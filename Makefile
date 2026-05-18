.PHONY: build dev clean frontend

build: frontend
	cargo build

dev: frontend
	cargo build

release: frontend
	cargo build --release

frontend: resources/excalidraw/dist/index.html

resources/excalidraw/dist/index.html: resources/excalidraw/package.json resources/excalidraw/src/main.tsx resources/excalidraw/index.html resources/excalidraw/vite.config.ts
	cd resources/excalidraw && npm install && npx vite build

clean:
	cargo clean
	rm -rf resources/excalidraw/dist resources/excalidraw/node_modules
