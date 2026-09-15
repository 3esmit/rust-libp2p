## 0.5.0

<!-- Update to libp2p-swarm v0.45.0 -->

- Update `rustls` to `0.23.45` to include the TLS 1.3 handshake boundary fix.
- Update the direct `rustls-webpki` dependency to `0.103.15`.

## 0.4.1

- Fix a panic caused by `rustls` parsing the libp2p TLS extension.
  See [PR 5498](https://github.com/libp2p/rust-libp2p/pull/5498).

## 0.4.0

- Upgrade `rustls` to `0.23`. See [PR 5385](https://github.com/libp2p/rust-libp2p/pull/5385)

## 0.3.0

- Migrate to `{In,Out}boundConnectionUpgrade` traits.
  See [PR 4695](https://github.com/libp2p/rust-libp2p/pull/4695).

## 0.2.1

- Switch from webpki to rustls-webpki.
  This is a part of the resolution of the [RUSTSEC-2023-0052].
  See [PR 4381].

[PR 4381]: https://github.com/libp2p/rust-libp2p/pull/4381
[RUSTSEC-2023-0052]: https://rustsec.org/advisories/RUSTSEC-2023-0052.html

## 0.2.0

- Raise MSRV to 1.65.
  See [PR 3715].

[PR 3715]: https://github.com/libp2p/rust-libp2p/pull/3715

## 0.1.0

- Promote to `v0.1.0`.

## 0.1.0-alpha.2

- Update to `libp2p-core` `v0.39.0`.

## 0.1.0-alpha

Initial release.
