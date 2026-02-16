export interface VerificationResult {
  valid: boolean;
  tessera_hash: string;
  manifest: {
    creator_pubkey: {
      ed25519: string;
      ml_dsa: string | null;
    };
    file_count: number;
  };
  signatures: {
    ed25519: "valid" | "invalid" | "missing";
    ml_dsa: "valid" | "invalid" | "missing";
  };
  files: Array<{
    path: string;
    status: "valid" | "invalid" | "missing";
    expected_hash: string;
    actual_hash: string | null;
  }>;
  unexpected_files: string[];
  errors: string[];
}

export type ProgressCallback = (
  current: number,
  total: number,
  file: string
) => void;
