use libp2p_kad::{GetRecordOk, KBucketDistance, PeerRecord, Record, ALPHA_VALUE, K_VALUE, U256};

#[test]
fn found_record_preserves_unboxed_constructor_and_match() {
    let record = PeerRecord {
        peer: None,
        record: Record::new(vec![1], vec![2, 3]),
    };
    let result = GetRecordOk::FoundRecord(record.clone());

    match result {
        GetRecordOk::FoundRecord(found) => {
            let found: PeerRecord = found;
            assert_eq!(found, record);
        }
        GetRecordOk::FinishedWithNoAdditionalRecord { .. } => panic!("expected record"),
    }
}

#[test]
fn distance_preserves_public_integer_constructor_and_operations() {
    let value = U256([9, 0, 0, 0]);
    let KBucketDistance(distance) = KBucketDistance(value);

    assert_eq!(distance, value);
    assert_eq!(value.integer_sqrt(), U256::from(3));
    assert_eq!(U256::from_big_endian(&value.to_big_endian()), value);
    assert_eq!(KBucketDistance(value).ilog2(), Some(3));
    assert_eq!(K_VALUE.get(), 20);
    assert_eq!(ALPHA_VALUE.get(), 3);
}
