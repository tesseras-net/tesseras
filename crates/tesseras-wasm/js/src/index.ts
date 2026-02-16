import type { VerificationResult, ProgressCallback } from "./types.js";

export type { VerificationResult, ProgressCallback };

export async function verifyTessera(
  archive: Uint8Array,
  onProgress?: ProgressCallback
): Promise<VerificationResult> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(
      new URL("./worker.js", import.meta.url),
      { type: "module" }
    );

    worker.onmessage = (event) => {
      const msg = event.data;

      switch (msg.type) {
        case "progress":
          onProgress?.(msg.current, msg.total, msg.file);
          break;
        case "result":
          worker.terminate();
          resolve(msg.result);
          break;
        case "error":
          worker.terminate();
          reject(new Error(msg.message));
          break;
      }
    };

    worker.onerror = (event) => {
      worker.terminate();
      reject(new Error(`Worker error: ${event.message}`));
    };

    // Init WASM in worker, then send archive
    const wasmUrl = new URL(
      "../../pkg/tesseras_wasm.js",
      import.meta.url
    ).href;
    worker.postMessage({ type: "init", wasmUrl });

    // Send archive with transfer (zero-copy)
    worker.postMessage(
      { type: "verify", archive },
      [archive.buffer]
    );
  });
}
