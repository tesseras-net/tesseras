use crate::error::RpcError;
use std::io::{Read, Write};

const MAX_FRAME_SIZE: u32 = 64 * 1024 * 1024; // 64 MiB

/// Write a length-prefixed MessagePack frame.
pub fn write_frame<W: Write, T: serde::Serialize>(writer: &mut W, msg: &T) -> Result<(), RpcError> {
    let payload = rmp_serde::to_vec(msg).map_err(|e| RpcError::Protocol(e.to_string()))?;
    let len = payload.len() as u32;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// Read a length-prefixed MessagePack frame.
pub fn read_frame<R: Read, T: serde::de::DeserializeOwned>(reader: &mut R) -> Result<T, RpcError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME_SIZE {
        return Err(RpcError::Protocol(format!(
            "frame too large: {len} bytes (max {MAX_FRAME_SIZE})"
        )));
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload)?;
    rmp_serde::from_slice(&payload).map_err(|e| RpcError::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{PublishState, Request, Response};
    use tesseras_core::ContentHash;

    #[test]
    fn frame_roundtrip_request() {
        let hash = ContentHash::new([0x11; 32]);
        let req = Request::Publish { hash };

        let mut buf = Vec::new();
        write_frame(&mut buf, &req).unwrap();

        let mut cursor = std::io::Cursor::new(&buf);
        let decoded: Request = read_frame(&mut cursor).unwrap();
        match decoded {
            Request::Publish { hash: h } => assert_eq!(h, hash),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn frame_roundtrip_response() {
        let hash = ContentHash::new([0x22; 32]);
        let resp = Response::Status {
            hash,
            state: PublishState::Healthy,
            fragments_total: 12,
            fragments_placed: 12,
            peers_holding: 5,
        };

        let mut buf = Vec::new();
        write_frame(&mut buf, &resp).unwrap();

        let mut cursor = std::io::Cursor::new(&buf);
        let decoded: Response = read_frame(&mut cursor).unwrap();
        match decoded {
            Response::Status {
                fragments_total,
                peers_holding,
                ..
            } => {
                assert_eq!(fragments_total, 12);
                assert_eq!(peers_holding, 5);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn frame_length_prefix_is_correct() {
        let req = Request::Fetch {
            hash: ContentHash::new([0x33; 32]),
        };
        let mut buf = Vec::new();
        write_frame(&mut buf, &req).unwrap();

        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(len as usize + 4, buf.len());
    }

    #[test]
    fn frame_rejects_oversized() {
        // Craft a length prefix claiming 128 MiB
        let mut buf = Vec::new();
        buf.extend_from_slice(&(128 * 1024 * 1024u32).to_be_bytes());
        buf.extend_from_slice(&[0; 64]); // junk payload

        let mut cursor = std::io::Cursor::new(&buf);
        let result: Result<Request, _> = read_frame(&mut cursor);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("frame too large"), "got: {err}");
    }
}
