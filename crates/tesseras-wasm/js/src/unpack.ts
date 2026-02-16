import { decompressSync, unzipSync } from "fflate";

export type FileMap = Map<string, Uint8Array>;

/**
 * Detect archive format and unpack to a file map.
 * Supports: .tar.gz (gzip + tar), .zip, .tar
 */
export function unpackArchive(data: Uint8Array): FileMap {
  // Gzip: \x1f\x8b
  if (data[0] === 0x1f && data[1] === 0x8b) {
    const decompressed = decompressSync(data);
    return unpackTar(decompressed);
  }

  // ZIP: PK\x03\x04
  if (
    data[0] === 0x50 &&
    data[1] === 0x4b &&
    data[2] === 0x03 &&
    data[3] === 0x04
  ) {
    return unpackZip(data);
  }

  // TAR: "ustar" at offset 257
  if (data.length > 262) {
    const magic = new TextDecoder().decode(data.slice(257, 262));
    if (magic === "ustar") {
      return unpackTar(data);
    }
  }

  throw new Error(
    "Unsupported archive format. Expected .tar.gz, .zip, or .tar"
  );
}

function unpackZip(data: Uint8Array): FileMap {
  const files = unzipSync(data);
  const map: FileMap = new Map();
  for (const [path, content] of Object.entries(files)) {
    // Skip directories (empty entries)
    if (content.length > 0) {
      map.set(normalizePath(path), content);
    }
  }
  return map;
}

function unpackTar(data: Uint8Array): FileMap {
  const map: FileMap = new Map();
  let offset = 0;

  while (offset < data.length - 512) {
    // Check for end-of-archive (two consecutive zero blocks)
    const header = data.slice(offset, offset + 512);
    if (header.every((b) => b === 0)) break;

    // Extract filename (bytes 0-99, null-terminated)
    const nameEnd = header.indexOf(0);
    const name = new TextDecoder()
      .decode(
        header.slice(0, nameEnd > 0 && nameEnd < 100 ? nameEnd : 100)
      )
      .trim();

    // Extract file size (bytes 124-135, octal ASCII)
    const sizeStr = new TextDecoder()
      .decode(header.slice(124, 136))
      .trim();
    const size = parseInt(sizeStr, 8) || 0;

    // Extract type flag (byte 156): '0' or '\0' = regular file
    const typeFlag = header[156];

    offset += 512; // move past header

    if ((typeFlag === 0x30 || typeFlag === 0x00) && size > 0 && name) {
      map.set(normalizePath(name), data.slice(offset, offset + size));
    }

    // Advance past file data (padded to 512-byte blocks)
    offset += Math.ceil(size / 512) * 512;
  }

  return map;
}

/**
 * Normalize path: strip leading tessera directory prefix if present.
 * "tessera-abc123/MANIFEST" -> "MANIFEST"
 * "tessera-abc123/memories/001/media.jpg" -> "memories/001/media.jpg"
 */
function normalizePath(path: string): string {
  const match = path.match(/^tessera-[a-z0-9]+\/(.+)$/);
  return match ? match[1] : path;
}
