import React, { useCallback, useRef } from 'react';
import ReactDOM from 'react-dom/client';
import { Excalidraw, serializeAsJSON } from '@excalidraw/excalidraw';
import '@excalidraw/excalidraw/index.css';

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let pendingData: { elements: any[]; appState: any } | null = null;

function App() {
    const apiRef = useRef<any>(null);

    const handleChange = useCallback((elements: readonly any[], appState: any, files: any) => {
        if (saveTimer) clearTimeout(saveTimer);
        saveTimer = setTimeout(() => {
            const serialized = serializeAsJSON(
                elements,
                appState,
                files || {},
                'local',
            );
            window.ipc.postMessage(JSON.stringify({
                type: 'save',
                data: JSON.parse(serialized),
            }));
        }, 1500);
    }, []);

    return (
        <Excalidraw
            excalidrawAPI={(api) => {
                apiRef.current = api;
                if (pendingData) {
                    api.updateScene(pendingData);
                    pendingData = null;
                }
                (window as any).__excalidraw_api = api;
            }}
            onChange={handleChange}
            theme="dark"
            UIOptions={{
                canvasActions: {
                    loadScene: false,
                    export: false,
                    saveToActiveFile: false,
                },
            }}
        />
    );
}

(window as any).__myco_load = function(jsonStr: string) {
    try {
        const parsed = JSON.parse(jsonStr);
        let elements: any[];
        let appState: any = {};

        if (parsed.elements) {
            elements = parsed.elements;
            appState = parsed.appState || {};
        } else if (typeof parsed === 'string') {
            const inner = JSON.parse(parsed);
            elements = inner.elements || [];
            appState = inner.appState || {};
        } else {
            elements = [];
        }

        const api = (window as any).__excalidraw_api;
        if (api) {
            api.updateScene({ elements, appState });
        } else {
            pendingData = { elements, appState };
        }
    } catch (e) {
        console.error('Failed to load canvas data:', e);
    }
};

document.addEventListener('keydown', (e) => {
    if (e.metaKey) {
        const appShortcuts = ['w', 'b', 'd', 'D', 't', ']', '['];
        if (appShortcuts.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
            window.ipc.postMessage(JSON.stringify({
                type: 'shortcut',
                key: e.key,
                shift: e.shiftKey,
            }));
        }
    }
}, true);

(window as any).__myco_set_focus = function(focused: boolean) {
    document.body.className = focused ? 'focused' : 'unfocused';
};

ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
