+++
title = "Fase 4: Verificar Sem Instalar Nada"
date = 2026-02-15T20:00:00+00:00
description = "Tesseras agora compila para WebAssembly — qualquer pessoa pode verificar integridade e autenticidade de uma tessera diretamente no navegador, sem instalar nenhum software."
+++

Confiança não deveria exigir instalação de software. Se alguém te envia uma tessera — um pacote de memórias preservadas — você deveria poder verificar que é genuína e não foi modificada sem baixar um app, criar uma conta, ou confiar em um servidor. É isso que o `tesseras-wasm` entrega: arraste um arquivo tessera para uma página web, e a verificação criptográfica acontece inteiramente no seu navegador.

## O que foi construído

**tesseras-wasm** — Um crate Rust que compila para WebAssembly via wasm-pack, expondo quatro funções stateless para JavaScript. O crate depende do `tesseras-core` para parsing do manifesto e chama primitivas criptográficas diretamente (blake3, ed25519-dalek) ao invés de depender do `tesseras-crypto`, que puxa bibliotecas pós-quânticas baseadas em C que não compilam para `wasm32-unknown-unknown`.

`parse_manifest` recebe os bytes brutos do MANIFEST (texto UTF-8 plano, não MessagePack), delega para `tesseras_core::manifest::Manifest::parse()`, e retorna uma string JSON com a chave pública Ed25519 do criador, caminhos dos arquivos de assinatura, e uma lista de arquivos com seus hashes BLAKE3 esperados, tamanhos e tipos MIME. Structs internas (`ManifestJson`, `CreatorPubkey`, `SignatureFiles`, `FileEntry`) são serializadas com serde_json. Os campos de chave pública ML-DSA e arquivo de assinatura estão presentes no contrato JSON mas definidos como `null` — prontos para quando a assinatura pós-quântica for implementada no lado nativo.

`hash_blake3` computa um hash BLAKE3 de bytes arbitrários e retorna uma string hexadecimal de 64 caracteres. É chamada uma vez por arquivo na tessera para verificar integridade contra o MANIFEST.

`verify_ed25519` recebe uma mensagem, uma assinatura de 64 bytes e uma chave pública de 32 bytes, constrói uma `ed25519_dalek::VerifyingKey`, e retorna se a assinatura é válida. A validação de comprimento retorna erros descritivos ("Ed25519 public key must be 32 bytes") ao invés de causar panic.

`verify_ml_dsa` é um stub que retorna um erro explicando que verificação ML-DSA ainda não está disponível. Isso é deliberado: o crate `ml-dsa` no crates.io está na v0.1.0-rc.7 (pré-release), e o `tesseras-crypto` usa `pqcrypto-dilithium` (CRYSTALS-Dilithium baseado em C) que é incompatível em nível de bytes com FIPS 204 ML-DSA. Ambos os lados precisam usar a mesma implementação em Rust puro antes que a verificação cruzada funcione. Verificação Ed25519 é suficiente — toda tessera é assinada com Ed25519.

Todas as quatro funções usam um padrão de duas camadas para testabilidade: funções internas retornam `Result<T, String>` e são testadas nativamente, enquanto wrappers finos `#[wasm_bindgen]` convertem erros para `JsError`. Isso evita que `JsError::new()` cause panic em targets não-WASM durante os testes.

O binário WASM compilado tem 109 KB bruto e 44 KB com gzip — bem abaixo do orçamento de 200 KB. O wasm-opt aplica otimização `-Oz` após o wasm-pack compilar com `opt-level = "z"`, LTO e uma única unidade de codegen.

**@tesseras/verify** — Um pacote npm TypeScript (`crates/tesseras-wasm/js/`) que orquestra a verificação no lado do navegador. A API pública é uma única função:

```typescript
async function verifyTessera(
  archive: Uint8Array,
  onProgress?: (current: number, total: number, file: string) => void
): Promise<VerificationResult>
```

O tipo `VerificationResult` fornece tudo que uma UI precisa: validade geral, hash da tessera, chaves públicas do criador, status das assinaturas (valid/invalid/missing para Ed25519 e ML-DSA), resultados de integridade por arquivo com hashes esperados e reais, uma lista de arquivos inesperados não presentes no MANIFEST, e um array de erros.

A descompactação de arquivos (`unpack.ts`) lida com três formatos: tar comprimido com gzip (detectado pelos magic bytes `\x1f\x8b`, descomprimido com fflate e depois parseado como tar), ZIP (magic `PK\x03\x04`, descompactado com `unzipSync` do fflate), e tar bruto (`ustar` no offset 257). Uma função `normalizePath` remove o prefixo `tessera-<hash>/` para que os caminhos internos correspondam às entradas do MANIFEST.

A verificação roda em um Web Worker (`worker.ts`) para manter a thread da UI responsiva. O worker inicializa o módulo WASM, descompacta o arquivo, parseia o MANIFEST, verifica a assinatura Ed25519 contra a chave pública do criador, depois faz hash de cada arquivo com BLAKE3 e compara com os valores esperados. Mensagens de progresso são transmitidas de volta para a thread principal após cada arquivo. Se qualquer assinatura é inválida, a verificação para imediatamente sem fazer hash dos arquivos — falhando rápido na verificação mais crítica.

O arquivo é transferido para o worker com zero-copy (`worker.postMessage({ type: "verify", archive }, [archive.buffer])`) para evitar duplicar arquivos de tessera potencialmente grandes na memória.

**Pipeline de build** — Três novos targets no justfile: `wasm-build` executa wasm-pack com `--target web --release` e otimiza com wasm-opt; `wasm-size` reporta o tamanho do binário bruto e com gzip; `test-wasm` executa a suíte de testes nativos.

**Testes** — 9 testes unitários nativos cobrem hashing BLAKE3 (entrada vazia, valor conhecido), verificação Ed25519 (assinatura válida, assinatura inválida, chave errada, comprimento de chave inválido), e parsing do MANIFEST (manifesto válido, UTF-8 inválido, lixo). 3 testes de integração WASM rodam em Chrome headless via `wasm-pack test --headless --chrome`, verificando que `hash_blake3`, `verify_ed25519` e `parse_manifest` funcionam corretamente quando compilados para `wasm32-unknown-unknown`.

## Decisões de arquitetura

- **Sem dependência do tesseras-crypto**: o crate WASM chama blake3 e ed25519-dalek diretamente. O `tesseras-crypto` depende do `pqcrypto-kyber` (ML-KEM baseado em C via pqcrypto-traits) que requer um toolchain de compilador C e não tem target wasm32. Dependendo apenas de crates Rust puros, o build WASM tem zero dependências C e compila sem problemas para WebAssembly.
- **ML-DSA adiado, não fingido**: ao invés de silenciosamente pular a verificação pós-quântica, o stub retorna um erro explícito. Isso garante que se uma tessera contiver uma assinatura ML-DSA, o resultado da verificação reportará `ml_dsa: "missing"` ao invés de fingir que foi verificada. O orquestrador JS lida com isso graciosamente — uma tessera é válida se Ed25519 passar e ML-DSA estiver ausente (ainda não implementado em nenhum dos lados).
- **Padrão de função interna**: `JsError` não pode ser construído em targets não-WASM (causa panic). Dividir cada função em `foo_inner() -> Result<T, String>` e `foo() -> Result<T, JsError>` permite que a suíte de testes nativa exercite toda a lógica sem tocar em tipos JavaScript. Os testes de integração WASM em Chrome headless testam a superfície completa do `#[wasm_bindgen]`.
- **Isolamento em Web Worker**: operações criptográficas (especialmente BLAKE3 sobre arquivos de mídia grandes) podem levar centenas de milissegundos. Rodar em um Worker previne travamentos na UI. O protocolo de progresso com streaming (`{ type: "progress", current, total, file }`) permite que a UI mostre uma barra de progresso durante a verificação de tesseras com muitos arquivos.
- **Transferência zero-copy**: `archive.buffer` é transferido para o Worker, não copiado. Para um arquivo tessera de 50 MB, isso evita dobrar o uso de memória durante a verificação.
- **MANIFEST em texto plano, não MessagePack**: o crate WASM parseia o mesmo formato de MANIFEST em texto plano que o CLI. Isso é por design — o MANIFEST é a Pedra de Rosetta da tessera, legível por qualquer pessoa com um editor de texto. A dependência `rmp-serde` no Cargo.toml não é usada e será removida.

## O que vem a seguir

- **Fase 4: Resiliência e Escala** — Empacotamento para sistemas operacionais (Alpine, Arch, Debian, FreeBSD, OpenBSD), CI no SourceHut e GitHub Actions, auditorias de segurança, explorador de tesseras no navegador em tesseras.net usando @tesseras/verify
- **Fase 5: Exploração e Cultura** — Navegador público de tesseras por era/localização/tema/idioma, curadoria institucional, integração com genealogia, exportação para mídia física (M-DISC, microfilme, papel livre de ácido com QR)

A verificação não exige mais confiança em software. Um arquivo tessera arrastado para um navegador é verificado com o mesmo rigor criptográfico do CLI — mesmos hashes BLAKE3, mesmas assinaturas Ed25519, mesmo parser de MANIFEST. A diferença é que agora qualquer pessoa pode fazer isso.
