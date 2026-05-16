import React from 'react';
import ReactDOM from 'react-dom/client';
import { Tldraw, getSnapshot, loadSnapshot } from 'tldraw';
import 'tldraw/tldraw.css';

let store: any = null;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let pendingSnapshot: any = null;

function App() {
    return (
        <Tldraw
            inferDarkMode
            onMount={(editor) => {
                store = editor.store;

                // Apply any snapshot that arrived before mount
                if (pendingSnapshot) {
                    loadSnapshot(store, pendingSnapshot);
                    pendingSnapshot = null;
                }

                // D-02: Auto-save with 1500ms debounce
                store.listen(() => {
                    if (saveTimer) clearTimeout(saveTimer);
                    saveTimer = setTimeout(() => {
                        const snapshot = getSnapshot(store);
                        window.ipc.postMessage(JSON.stringify({
                            type: 'save',
                            data: snapshot
                        }));
                    }, 1500);
                }, { scope: 'document', source: 'user' });
            }}
        />
    );
}

// Receive load commands from Rust via evaluate_script
(window as any).__myco_load = function(jsonStr: string) {
    const data = JSON.parse(jsonStr);
    if (store) {
        loadSnapshot(store, data);
    } else {
        pendingSnapshot = data;
    }
};

// D-14: Forward Cmd-key events to Rust before TLDraw handles them
document.addEventListener('keydown', (e) => {
    if (e.metaKey) {
        const appShortcuts = ['w', 'b', 'd', 'D', 't', ']', '['];
        if (appShortcuts.includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
            window.ipc.postMessage(JSON.stringify({
                type: 'shortcut',
                key: e.key,
                shift: e.shiftKey
            }));
        }
    }
}, true);

// D-16: Focus/blur handling for desaturation
(window as any).__myco_set_focus = function(focused: boolean) {
    document.body.className = focused ? 'focused' : 'unfocused';
};

ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
