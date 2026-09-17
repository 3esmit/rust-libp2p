#![cfg(feature = "tokio")]

use libp2p_identity::PeerId;
use libp2p_webrtc::tokio::Error;

#[test]
fn peer_mismatch_preserves_public_error_payloads() {
    let expected = PeerId::random();
    let got = PeerId::random();
    let error = Error::InvalidPeerID { expected, got };

    assert_eq!(
        error.to_string(),
        format!("invalid peer ID (expected {expected}, got {got})")
    );
    match error {
        Error::InvalidPeerID {
            expected: actual_expected,
            got: actual_got,
        } => {
            let actual_expected: PeerId = actual_expected;
            let actual_got: PeerId = actual_got;
            assert_eq!(actual_expected, expected);
            assert_eq!(actual_got, got);
        }
        _ => panic!("expected peer mismatch"),
    }
}
