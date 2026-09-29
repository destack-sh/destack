/** Current rendered bodies and documentation metadata. */
export const assets: Record<string, () => Promise<string>> = {
    "/_content/docs/architecture/accounts/access/index.json": () =>
        import("../../public/_content/docs/architecture/accounts/access/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/accounts/account/index.json": () =>
        import("../../public/_content/docs/architecture/accounts/account/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/accounts/audit/index.json": () =>
        import("../../public/_content/docs/architecture/accounts/audit/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/accounts/index.json": () =>
        import("../../public/_content/docs/architecture/accounts/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/accounts/organisation/index.json": () =>
        import("../../public/_content/docs/architecture/accounts/organisation/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/apps/cli/index.json": () =>
        import("../../public/_content/docs/architecture/apps/cli/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/apps/desktop/index.json": () =>
        import("../../public/_content/docs/architecture/apps/desktop/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/apps/home/index.json": () =>
        import("../../public/_content/docs/architecture/apps/home/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/apps/index.json": () =>
        import("../../public/_content/docs/architecture/apps/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/apps/view/index.json": () =>
        import("../../public/_content/docs/architecture/apps/view/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/data/bucket/index.json": () =>
        import("../../public/_content/docs/architecture/data/bucket/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/data/db/index.json": () =>
        import("../../public/_content/docs/architecture/data/db/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/data/index.json": () =>
        import("../../public/_content/docs/architecture/data/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/data/vault/index.json": () =>
        import("../../public/_content/docs/architecture/data/vault/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/hosts/daemon/index.json": () =>
        import("../../public/_content/docs/architecture/hosts/daemon/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/hosts/index.json": () =>
        import("../../public/_content/docs/architecture/hosts/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/hosts/sandbox/index.json": () =>
        import("../../public/_content/docs/architecture/hosts/sandbox/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/hosts/telemetry/index.json": () =>
        import("../../public/_content/docs/architecture/hosts/telemetry/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/hosts/update/index.json": () =>
        import("../../public/_content/docs/architecture/hosts/update/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/index.json": () =>
        import("../../public/_content/docs/architecture/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/overview/index.json": () =>
        import("../../public/_content/docs/architecture/overview/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/overview/space/index.json": () =>
        import("../../public/_content/docs/architecture/overview/space/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/overview/universe/index.json": () =>
        import("../../public/_content/docs/architecture/overview/universe/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/services/gateway/index.json": () =>
        import("../../public/_content/docs/architecture/services/gateway/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/services/index.json": () =>
        import("../../public/_content/docs/architecture/services/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/services/service/index.json": () =>
        import("../../public/_content/docs/architecture/services/service/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/services/setting/index.json": () =>
        import("../../public/_content/docs/architecture/services/setting/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/source/build/index.json": () =>
        import("../../public/_content/docs/architecture/source/build/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/source/index.json": () =>
        import("../../public/_content/docs/architecture/source/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/source/package/index.json": () =>
        import("../../public/_content/docs/architecture/source/package/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/source/registry/index.json": () =>
        import("../../public/_content/docs/architecture/source/registry/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/architecture/source/repository/index.json": () =>
        import("../../public/_content/docs/architecture/source/repository/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/index.json": () =>
        import("../../public/_content/docs/index.json?raw").then((module) => module.default),
    "/_content/docs/library/index.json": () =>
        import("../../public/_content/docs/library/index.json?raw").then(
            (module) => module.default,
        ),
    "/_content/docs/setup/index.json": () =>
        import("../../public/_content/docs/setup/index.json?raw").then((module) => module.default),
    "/_content/html/036f09ab33b0f5a560eeb07fdc5f76eef8828358f71afcddb0cadb31e223fad4.html": () =>
        import("../../public/_content/html/036f09ab33b0f5a560eeb07fdc5f76eef8828358f71afcddb0cadb31e223fad4.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/11f439f453037c4a20476663e23e8cec966f1a6e79b206b518765433031f9fe0.html": () =>
        import("../../public/_content/html/11f439f453037c4a20476663e23e8cec966f1a6e79b206b518765433031f9fe0.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/1244bfa6d3dfc7b7881797fa0f4e72d8dd4bbbbb3221ed754f19667a9f967085.html": () =>
        import("../../public/_content/html/1244bfa6d3dfc7b7881797fa0f4e72d8dd4bbbbb3221ed754f19667a9f967085.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/134c071413ee8eb107939ec5dd861545f7e01c59da8af58e62ba30d4f8a485a4.html": () =>
        import("../../public/_content/html/134c071413ee8eb107939ec5dd861545f7e01c59da8af58e62ba30d4f8a485a4.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/15111f946c44c704dd901f631bb7d9373a5484b82c4a4db1ce108fc3bd16dac0.html": () =>
        import("../../public/_content/html/15111f946c44c704dd901f631bb7d9373a5484b82c4a4db1ce108fc3bd16dac0.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/19dbfb5e2d89d9531355e0614164231de5526b6a24f8117bf99a68fa4c7cd3a0.html": () =>
        import("../../public/_content/html/19dbfb5e2d89d9531355e0614164231de5526b6a24f8117bf99a68fa4c7cd3a0.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/1a1f54b48064e262bd93c7787861109986076b9f7b45668a4bf16f07295d6341.html": () =>
        import("../../public/_content/html/1a1f54b48064e262bd93c7787861109986076b9f7b45668a4bf16f07295d6341.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/1c11b7717be6c23d03a1b50115d45cace050ca5a94c4f707537700009e6b1ae5.html": () =>
        import("../../public/_content/html/1c11b7717be6c23d03a1b50115d45cace050ca5a94c4f707537700009e6b1ae5.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/1d1fbd4a0605e24f954fc688516aea9a45e486eea72280993801cca4b3351157.html": () =>
        import("../../public/_content/html/1d1fbd4a0605e24f954fc688516aea9a45e486eea72280993801cca4b3351157.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/2d488adb1cdc422a21afb7426bed2c7f817b7823eebb00601b4819e611528525.html": () =>
        import("../../public/_content/html/2d488adb1cdc422a21afb7426bed2c7f817b7823eebb00601b4819e611528525.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/383b42f360f0cbbe7663d3be57f7e09ae435a7b99b2e7fdec4481d887c8b4150.html": () =>
        import("../../public/_content/html/383b42f360f0cbbe7663d3be57f7e09ae435a7b99b2e7fdec4481d887c8b4150.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/42db6385093295b194406c42e13fbdc90666a3566b4d95bcff71302b01a2af96.html": () =>
        import("../../public/_content/html/42db6385093295b194406c42e13fbdc90666a3566b4d95bcff71302b01a2af96.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/43fdd83b1ebc6df6c7326ba19bf0a311d72ff1fb249bbf55a47d60aad04f49e9.html": () =>
        import("../../public/_content/html/43fdd83b1ebc6df6c7326ba19bf0a311d72ff1fb249bbf55a47d60aad04f49e9.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/4978439ffda1d01769dfe1740ed78df8ff9c1fac465ed8fb8f2f546715f2d461.html": () =>
        import("../../public/_content/html/4978439ffda1d01769dfe1740ed78df8ff9c1fac465ed8fb8f2f546715f2d461.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/5138f3ddeb3badf1bdeeb45013218959642f3a1cae58513afbc229676e1aad76.html": () =>
        import("../../public/_content/html/5138f3ddeb3badf1bdeeb45013218959642f3a1cae58513afbc229676e1aad76.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/53db905b9ae361091378e99a2f2a032776779f884d5de09b746f5b2976674335.html": () =>
        import("../../public/_content/html/53db905b9ae361091378e99a2f2a032776779f884d5de09b746f5b2976674335.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/57351aa407aad9044f3e7944332fa78a81d11d2a0f7bd587b90d6bb2cf1e241a.html": () =>
        import("../../public/_content/html/57351aa407aad9044f3e7944332fa78a81d11d2a0f7bd587b90d6bb2cf1e241a.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/5c96278082f0c0e922e0d7a7838109296f4c5f92a0bf233130075aa22899ebec.html": () =>
        import("../../public/_content/html/5c96278082f0c0e922e0d7a7838109296f4c5f92a0bf233130075aa22899ebec.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/5cac92950e581a7a943fd56a290456ab9b5506192446e2aed86791e0448fcbc1.html": () =>
        import("../../public/_content/html/5cac92950e581a7a943fd56a290456ab9b5506192446e2aed86791e0448fcbc1.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/60137b893857b0b1c506b9bf619fe82f3bb91de8f271c293887ce5e0b0fce1b2.html": () =>
        import("../../public/_content/html/60137b893857b0b1c506b9bf619fe82f3bb91de8f271c293887ce5e0b0fce1b2.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/6c6f87cdcab7d9cd79ded5c4f4bb0e6c420d25e9ffbce0daada490c889befc83.html": () =>
        import("../../public/_content/html/6c6f87cdcab7d9cd79ded5c4f4bb0e6c420d25e9ffbce0daada490c889befc83.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/6e0eda15a693b80e6165762e142090fa75368aabe029385db42146f74896cd64.html": () =>
        import("../../public/_content/html/6e0eda15a693b80e6165762e142090fa75368aabe029385db42146f74896cd64.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/72b9cec1813a5e699c79ef33a22108361b10492735674ccc7667ea61ecd33148.html": () =>
        import("../../public/_content/html/72b9cec1813a5e699c79ef33a22108361b10492735674ccc7667ea61ecd33148.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/72d65d6deeeee457cffa756fdd904e2ab5394986330c9f5d417cae6a44e88ba5.html": () =>
        import("../../public/_content/html/72d65d6deeeee457cffa756fdd904e2ab5394986330c9f5d417cae6a44e88ba5.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/7559e30b5e4bab717d3e0d98b866425b2ce611df56c61ec6dc4dacfae9530d09.html": () =>
        import("../../public/_content/html/7559e30b5e4bab717d3e0d98b866425b2ce611df56c61ec6dc4dacfae9530d09.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/8c45413898a36c49bc72d9cb5b9206bac10b475a1dc3b49381f877393d7f748d.html": () =>
        import("../../public/_content/html/8c45413898a36c49bc72d9cb5b9206bac10b475a1dc3b49381f877393d7f748d.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/96f2771480f73bb74d88801cf172f361147d94f3b89bfa623b5d59e9aed78ced.html": () =>
        import("../../public/_content/html/96f2771480f73bb74d88801cf172f361147d94f3b89bfa623b5d59e9aed78ced.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/9a1d6e066ed65fe4a5f132b5bd93e7b114c7250b99f2abb638ec346335bddf6e.html": () =>
        import("../../public/_content/html/9a1d6e066ed65fe4a5f132b5bd93e7b114c7250b99f2abb638ec346335bddf6e.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/a135436f0193bfab4f2daa65f355be38495033d48f786b26904adea9cfbac9e5.html": () =>
        import("../../public/_content/html/a135436f0193bfab4f2daa65f355be38495033d48f786b26904adea9cfbac9e5.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/b5f1116dc7d69eaeef69cbc6502acc808daacdaeff7f97d5ebca5710cef3122e.html": () =>
        import("../../public/_content/html/b5f1116dc7d69eaeef69cbc6502acc808daacdaeff7f97d5ebca5710cef3122e.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/b774047e1fb0e518b4c781bb693a2b53b29e25eb78c30f7842afea12d4be1996.html": () =>
        import("../../public/_content/html/b774047e1fb0e518b4c781bb693a2b53b29e25eb78c30f7842afea12d4be1996.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/c5e41f8eb12a10863b667d2d531d4ea5537d9f3c39eeb9ed8d3a2465f7fdcc33.html": () =>
        import("../../public/_content/html/c5e41f8eb12a10863b667d2d531d4ea5537d9f3c39eeb9ed8d3a2465f7fdcc33.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/c83bc67ceee175ce8d3388fe67405e1c2a45330ff2eba0797f96e320c88270f6.html": () =>
        import("../../public/_content/html/c83bc67ceee175ce8d3388fe67405e1c2a45330ff2eba0797f96e320c88270f6.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/cb46d586e1ebd8cca29377b3c5f498c79254aec4d2cf39ed0bc85bfcae8df615.html": () =>
        import("../../public/_content/html/cb46d586e1ebd8cca29377b3c5f498c79254aec4d2cf39ed0bc85bfcae8df615.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/d7c313b28e13ca7df144674bd1aed120d5402efea4eafe2ff431df88e4685377.html": () =>
        import("../../public/_content/html/d7c313b28e13ca7df144674bd1aed120d5402efea4eafe2ff431df88e4685377.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/e9e8780e3d09f5b5feb49d745d417da3d90fe46bda3cbaba70c082c67a980daf.html": () =>
        import("../../public/_content/html/e9e8780e3d09f5b5feb49d745d417da3d90fe46bda3cbaba70c082c67a980daf.html?raw").then(
            (module) => module.default,
        ),
    "/_content/html/f34147302baac5aa6ece51aa9da62e223dcea8c77e8874e8d06aa9505842438a.html": () =>
        import("../../public/_content/html/f34147302baac5aa6ece51aa9da62e223dcea8c77e8874e8d06aa9505842438a.html?raw").then(
            (module) => module.default,
        ),
};
