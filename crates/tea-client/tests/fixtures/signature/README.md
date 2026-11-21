# Signature test vectors

Made with OpenSSL 3.6.4 on 2026-11-21. Only the public keys and signatures are
kept; the private keys were generated for this purpose and discarded.

```sh
printf 'libtea signature fixture\n' > artifact.txt
openssl genpkey -algorithm ed25519 -out ed.key
openssl pkey -in ed.key -pubout -out ed25519.pub.pem
openssl pkeyutl -sign -inkey ed.key -rawin -in artifact.txt -out artifact.ed25519.sig
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out p.key
openssl pkey -in p.key -pubout -out p256.pub.pem
openssl dgst -sha256 -sign p.key -out artifact.p256.sig artifact.txt
```

Fingerprints (`openssl pkey -pubin -in <key> -outform DER | sha256sum`):

- `ed25519.pub.pem`: `9797c6e083b023da1acb7b83fd4635033203c436f2562c00012191dc3670b20f`
- `p256.pub.pem`: `94abfadd4116e169226fb49f55625d71d44a724b2fcc52688e51f0d3e44c54f0`
