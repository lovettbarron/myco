import React, { useCallback, useRef } from 'react';
import ReactDOM from 'react-dom/client';
import { Excalidraw, serializeAsJSON } from '@excalidraw/excalidraw';
import '@excalidraw/excalidraw/index.css';

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let userHasInteracted = false;
let loadComplete = false;

// Parse init data injected into HTML by Rust custom protocol handler
let initialSceneData: { elements: any[]; appState: any } | undefined;
if ((window as any).__myco_init_data) {
    try {
        const parsed = JSON.parse((window as any).__myco_init_data);
        initialSceneData = {
            elements: parsed.elements || [],
            appState: parsed.appState || {},
        };
        loadComplete = true;
    } catch (e) {
        console.error('Failed to parse canvas init data:', e);
    }
}

function canSave(): boolean {
    if (!userHasInteracted) return false;
    if ((window as any).__myco_load_pending && !loadComplete) return false;
    return true;
}

function scheduleSave(api: any) {
    if (!canSave() || !api) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
        const elements = api.getSceneElements();
        const appState = api.getAppState();
        const files = api.getFiles();
        const serialized = serializeAsJSON(elements, appState, files || {}, 'local');
        window.ipc.postMessage(JSON.stringify({ type: 'save', data: JSON.parse(serialized) }));
    }, 1500);
}

function App() {
    const apiRef = useRef<any>(null);

    const handleChange = useCallback(() => { scheduleSave(apiRef.current); }, []);
    const handlePointerDown = useCallback(() => { userHasInteracted = true; }, []);
    const handlePointerUp = useCallback(() => { scheduleSave(apiRef.current); }, []);

    return (
        <Excalidraw
            excalidrawAPI={(api) => {
                apiRef.current = api;
                (window as any).__excalidraw_api = api;
            }}
            initialData={initialSceneData}
            onChange={handleChange}
            onPointerDown={handlePointerDown}
            onPointerUp={handlePointerUp}
            theme="dark"
            UIOptions={{ canvasActions: { loadScene: false, export: false, saveToActiveFile: false } }}
        />
    );
}

(window as any).__myco_load = function(jsonStr: string) {
    try {
        const parsed = JSON.parse(jsonStr);
        let elements: any[] = [];
        let appState: any = {};
        if (parsed.elements) { elements = parsed.elements; appState = parsed.appState || {}; }
        else if (typeof parsed === 'string') { const inner = JSON.parse(parsed); elements = inner.elements || []; appState = inner.appState || {}; }

        const api = (window as any).__excalidraw_api;
        if (api) { api.updateScene({ elements, appState }); }
    } catch (e) { console.error('Failed to load canvas data:', e); }
    loadComplete = true;
};

document.addEventListener('keydown', (e) => {
    if (e.metaKey) {
        const appShortcuts = ['w', 'b', 'd', 'D', 't', ']', '['];
        if (appShortcuts.includes(e.key)) {
            e.preventDefault(); e.stopPropagation();
            window.ipc.postMessage(JSON.stringify({ type: 'shortcut', key: e.key, shift: e.shiftKey }));
        }
    }
}, true);

(window as any).__myco_set_focus = function(focused: boolean) {
    document.body.className = focused ? 'focused' : 'unfocused';
};

ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
