// Worker receives archive bytes, runs verification, posts progress + result.
import { unpackArchive } from "./unpack.js";
import type { VerificationResult } from "./types.js";

// WASM module will be initialized when the worker starts
let wasm: any;

type WorkerMessage =
  | { type: "verify"; archive: Uint8Array }
  | { type: "init"; wasmUrl: string };

type WorkerResponse =
  | { type: "progress"; current: number; total: number; file: string }
  | { type: "result"; result: VerificationResult }
  | { type: "error"; message: string };

self.onmessage = async (event: MessageEvent<WorkerMessage>) => {
  const msg = event.data;

  if (msg.type === "init") {
    try {
      const wasmModule = await import(msg.wasmUrl);
      await wasmModule.default();
      wasm = wasmModule;
      return;
    } catch (e) {
      post({ type: "error", message: `Failed to init WASM: ${e}` });
      return;
    }
  }

  if (msg.type === "verify") {
    try {
      const result = await verify(msg.archive);
      post({ type: "result", result });
    } catch (e) {
      post({ type: "error", message: `Verification failed: ${e}` });
    }
  }
};

function post(msg: WorkerResponse) {
  self.postMessage(msg);
}

async function verify(
  archiveBytes: Uint8Array
): Promise<VerificationResult> {
  const errors: string[] = [];
  const files: VerificationResult["files"] = [];
  let unexpectedFiles: string[] = [];

  // 1. Unpack archive
  let fileMap;
  try {
    fileMap = unpackArchive(archiveBytes);
  } catch (e) {
    return errorResult(`Archive unpacking failed: ${e}`);
  }

  // 2. Find and parse MANIFEST
  const manifestBytes = fileMap.get("MANIFEST");
  if (!manifestBytes) {
    return errorResult("MANIFEST not found in archive");
  }

  let manifest;
  try {
    const jsonStr = wasm.parse_manifest(manifestBytes);
    manifest = JSON.parse(jsonStr);
  } catch (e) {
    return errorResult(`Failed to parse MANIFEST: ${e}`);
  }

  // 3. Verify Ed25519 signature
  let ed25519Status: "valid" | "invalid" | "missing" = "missing";
  const ed25519SigBytes = fileMap.get(manifest.signature_files.ed25519);
  if (ed25519SigBytes) {
    try {
      const pubkeyBytes = hexToBytes(manifest.creator_pubkey.ed25519);
      const valid = wasm.verify_ed25519(
        manifestBytes,
        ed25519SigBytes,
        pubkeyBytes
      );
      ed25519Status = valid ? "valid" : "invalid";
    } catch (e) {
      ed25519Status = "invalid";
      errors.push(`Ed25519 verification error: ${e}`);
    }
  }

  // 4. Verify ML-DSA signature (skip if null)
  let mlDsaStatus: "valid" | "invalid" | "missing" = "missing";
  if (manifest.signature_files.ml_dsa) {
    const mlDsaSigBytes = fileMap.get(manifest.signature_files.ml_dsa);
    if (mlDsaSigBytes && manifest.creator_pubkey.ml_dsa) {
      try {
        const pubkeyBytes = hexToBytes(manifest.creator_pubkey.ml_dsa);
        const valid = wasm.verify_ml_dsa(
          manifestBytes,
          mlDsaSigBytes,
          pubkeyBytes
        );
        mlDsaStatus = valid ? "valid" : "invalid";
      } catch (e) {
        mlDsaStatus = "invalid";
        errors.push(`ML-DSA verification error: ${e}`);
      }
    }
  }

  // 5. Early exit: any present signature invalid -> stop
  if (ed25519Status === "invalid" || mlDsaStatus === "invalid") {
    return {
      valid: false,
      tessera_hash: "",
      manifest: {
        creator_pubkey: manifest.creator_pubkey,
        file_count: manifest.files.length,
      },
      signatures: { ed25519: ed25519Status, ml_dsa: mlDsaStatus },
      files: [],
      unexpected_files: [],
      errors: [...errors, "Verification stopped: signature invalid"],
    };
  }

  // Both missing -> invalid (unsigned tessera)
  if (ed25519Status === "missing" && mlDsaStatus === "missing") {
    return {
      valid: false,
      tessera_hash: "",
      manifest: {
        creator_pubkey: manifest.creator_pubkey,
        file_count: manifest.files.length,
      },
      signatures: { ed25519: "missing", ml_dsa: "missing" },
      files: [],
      unexpected_files: [],
      errors: ["No signatures found — tessera is unsigned"],
    };
  }

  // 6. Verify each file hash
  const total = manifest.files.length;

  // Track known non-file paths (MANIFEST, signatures, identity/, decode/, schema/)
  const knownPaths = new Set<string>(["MANIFEST"]);
  if (manifest.signature_files.ed25519)
    knownPaths.add(manifest.signature_files.ed25519);
  if (manifest.signature_files.ml_dsa)
    knownPaths.add(manifest.signature_files.ml_dsa);

  for (let i = 0; i < manifest.files.length; i++) {
    const entry = manifest.files[i];
    knownPaths.add(entry.path);

    const fileBytes = fileMap.get(entry.path);
    if (!fileBytes) {
      files.push({
        path: entry.path,
        status: "missing",
        expected_hash: entry.hash,
        actual_hash: null,
      });
    } else {
      const actualHash = wasm.hash_blake3(fileBytes);
      const valid = actualHash === entry.hash;
      files.push({
        path: entry.path,
        status: valid ? "valid" : "invalid",
        expected_hash: entry.hash,
        actual_hash: actualHash,
      });
    }

    post({
      type: "progress",
      current: i + 1,
      total,
      file: entry.path,
    });
  }

  // 7. Collect unexpected files
  unexpectedFiles = [...fileMap.keys()].filter(
    (path) => !knownPaths.has(path)
  );

  // 8. Compute overall validity
  const allFilesValid = files.every((f) => f.status === "valid");
  const signaturesOk =
    ed25519Status === "valid" &&
    (mlDsaStatus === "valid" || mlDsaStatus === "missing");

  // Compute tessera hash from MANIFEST content
  const tesseraHash = wasm.hash_blake3(manifestBytes);

  return {
    valid: allFilesValid && signaturesOk,
    tessera_hash: tesseraHash,
    manifest: {
      creator_pubkey: manifest.creator_pubkey,
      file_count: manifest.files.length,
    },
    signatures: { ed25519: ed25519Status, ml_dsa: mlDsaStatus },
    files,
    unexpected_files: unexpectedFiles,
    errors,
  };
}

function errorResult(message: string): VerificationResult {
  return {
    valid: false,
    tessera_hash: "",
    manifest: {
      creator_pubkey: { ed25519: "", ml_dsa: null },
      file_count: 0,
    },
    signatures: { ed25519: "missing", ml_dsa: "missing" },
    files: [],
    unexpected_files: [],
    errors: [message],
  };
}

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substring(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}
