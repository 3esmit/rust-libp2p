use anyhow::Result;

fn identity_pem() -> Result<(String, String)> {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    Ok((
        certificate.serialize_pem()?,
        certificate.serialize_private_key_pem(),
    ))
}

#[test]
fn reqwest_rustls_accepts_certificate_bundles_and_identity_orders() -> Result<()> {
    let (certificate, key) = identity_pem()?;
    let second_certificate = rcgen::generate_simple_self_signed(vec!["example.test".to_owned()])?;
    let bundle = format!("{certificate}{}", second_certificate.serialize_pem()?);

    assert_eq!(
        reqwest::Certificate::from_pem_bundle(bundle.as_bytes())?.len(),
        2
    );

    for identity in [format!("{key}{certificate}"), format!("{certificate}{key}")] {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .identity(reqwest::Identity::from_pem(identity.as_bytes())?)
            .build()?;
        let _ = client;
    }
    Ok(())
}

#[test]
fn reqwest_rustls_rejects_missing_or_malformed_identity_material() -> Result<()> {
    let (certificate, key) = identity_pem()?;

    assert!(reqwest::Identity::from_pem(certificate.as_bytes()).is_err());
    assert!(reqwest::Identity::from_pem(key.as_bytes()).is_err());
    assert!(reqwest::Identity::from_pem(b"not a PEM identity").is_err());
    Ok(())
}
