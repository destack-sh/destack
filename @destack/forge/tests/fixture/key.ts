/** A fixed RS256 test key of the fixture GitHub App, as PKCS #8, since generating RSA keys takes about 100 ms. */
const PRIVATE_KEY =
    "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCZYGBaXnG/aT61p8aryFLQSkP9iXHIIB8swHMKojFDuShYUIrCPQy/QlKwExWvbuqs1WP7E48zo2PbnbPLGOa6IDoq2wtNeWxhveOXn8BROMPIZ+qAXlsNfBjaNpKtuQUyJDm+7TcaHo0dLe7oCSu3snQw19JNFuLfMKM6pazvS/XvbA6O//WIPlrSu3sVjuuw3qigPgcjz09Y3vdG+CbZ/49Jtev/ti6ojCS5rhmRmVUEx15E1Wp+bZ3kOl+OB2ov+sO3+txOQyfQJ1pIUD+zit+kJSVzns8izFoMCd0J8pAryBoGAVZZbJXfiPwpUFou6rVkNGTvqRKYoU0j4YUrAgMBAAECggEAAfp1nCFuNZlMzQLjgUcpgNMVrBydgleeQQJI5xHQ4JklyaD+206zE13aTDEsgxdk/AEMUiBeRte9I0sIpIiPnVOt4XLzEjBdDgaluHEXau+7vo3V2h8JPeTJVjn3MMC5wjHkvfKkJ38Ri5c0D1n94PleU/BTd0wXniulGDLoTWFWXPkP9tUOLrI/V0MDPn3o1QkyaLfCtHR3t9TedqRA7qwlHBKBbSoK0MERVnvdUgJEPgOCtLGp0zFHWtlC+krHI0ROo47AEYB3X/oGM7T/OmYDOEdvQdF0mASdn/ICHIg/rIqcPuN7xpc+kzVN9aEIYIsLc77k//v3VYqNvY9YgQKBgQDL9lTGy9uEfTUQEhQjLIuay/ZFq4G9RKq2pAXtBj35G6fjXoPji/Pf8qNRFNj5HV22bAzRdfzmRhqMdSy5kyWN2+85uokGbJ5hfsaa4TY+ZdGDlq9DmCr+tu4RH4IXj2ioJ9W/lbTLHN8J87AWwvxSSj4pwgiNo7nLFxwXV0KKgQKBgQDAghErwglhkfBoig1VZgOHV2s0Vk0JIo0ocm27GQ3DT6P27KSRtHrSBxjQduQ/iHb2jKBaG9JXqsXmeuyctA2Fq/rU94h4XT76u9bhGVL1OMmQuLMHOo7nnsjueBTeJ8+p2qixcMmW23bGkwBZdji1wQC8OvuFIyRMegHgmKSBqwKBgQCYnVNGBRfWhRGFWuGyiAxV4bP+4ZwMkSrjysucVYVdtnHjUb+CLiBnO9k0PFM14+FRNHxK5uw7Kc2Ht82ldhMtmJniKk+JfRDthrz4+vdprSoeOkweWegh+6MbMOZY1rfKyzPHTS+go5PIkIz8Gx5OPLtS55sa0J2e90XPuT1TAQKBgASt5K70m3fErVxNJr/Rp/pNNKzkACFdtqKqDWLD3CLIN8sTu3hTM70RMRg4G5ozvfiGOIuYK4e3/fF5QXEZq055fLJahfWvBo3frPxxuiN9yuB6rNdbJAgbQvUQ9CjmPNT7HKibfjrtgLiY8CQ8jGpWk6b/pAd7cDFvscX8z9rbAoGBAMCrBv78RspBB/AziQ24/Xbu6wuJCmVylDly28rJQ3KiS1TTWoK+dzyd4QCs1WeuFY4acVRtF0Dh4Ce/zzkIY76Q45Kz00Ybz98O50VaWkMRlpCujDlLSLqHjiwbW63t1PRD8bjQgvc7pxo765ufc58Iz53NpWFgT4srZ9QM5F+U";

/** The fixture GitHub App's public key, as SPKI. */
const PUBLIC_KEY =
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAmWBgWl5xv2k+tafGq8hS0EpD/YlxyCAfLMBzCqIxQ7koWFCKwj0Mv0JSsBMVr27qrNVj+xOPM6Nj252zyxjmuiA6KtsLTXlsYb3jl5/AUTjDyGfqgF5bDXwY2jaSrbkFMiQ5vu03Gh6NHS3u6Akrt7J0MNfSTRbi3zCjOqWs70v172wOjv/1iD5a0rt7FY7rsN6ooD4HI89PWN73Rvgm2f+PSbXr/7YuqIwkua4ZkZlVBMdeRNVqfm2d5DpfjgdqL/rDt/rcTkMn0CdaSFA/s4rfpCUlc57PIsxaDAndCfKQK8gaBgFWWWyV34j8KVBaLuq1ZDRk76kSmKFNI+GFKwIDAQAB";

/** The RS256 parameters of the fixture GitHub App's keys. */
const RS256 = { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" } as const;

/** Import the fixture GitHub App's private key. */
export function githubPrivateKey(): Promise<CryptoKey> {
    return crypto.subtle.importKey("pkcs8", Uint8Array.fromBase64(PRIVATE_KEY), RS256, false, [
        "sign",
    ]);
}

/** Import the fixture GitHub App's public key. */
export function githubPublicKey(): Promise<CryptoKey> {
    return crypto.subtle.importKey("spki", Uint8Array.fromBase64(PUBLIC_KEY), RS256, false, [
        "verify",
    ]);
}
