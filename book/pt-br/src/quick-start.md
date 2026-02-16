# Início Rápido

Este tutorial guia você por um fluxo completo: criar uma identidade, construir uma tessera a partir de arquivos, verificá-la, exportá-la e publicá-la na rede.

## 1. Inicializar sua identidade

Primeiro, configure sua identidade local e banco de dados:

```bash
tes init
```

```
Generated Ed25519 identity
Generating encryption keypair (X25519 + ML-KEM-768)...
Generated encryption keypair
Database initialized
Config written to /home/user/.local/share/tesseras/config.toml
Tesseras initialized at /home/user/.local/share/tesseras
```

Isso cria sua identidade criptográfica (Ed25519 para assinatura, X25519 + ML-KEM-768 para criptografia), um banco de dados SQLite, armazenamento de blobs e um arquivo de configuração padrão em `~/.local/share/tesseras/`.

## 2. Preparar seus arquivos

Crie um diretório com as memórias que deseja preservar:

```bash
mkdir minhas-memorias
cp ~/fotos/jantar-familia.jpg minhas-memorias/
cp ~/fotos/jardim.jpg minhas-memorias/
echo "Uma tarde quente de domingo com a família." > minhas-memorias/reflexao.txt
```

Formatos suportados: `.jpg`, `.jpeg`, `.png` (imagens), `.wav` (áudio), `.webm` (vídeo), `.txt` (texto).

## 3. Pré-visualizar com dry run

Veja o que seria incluído sem criar nada:

```bash
tes create minhas-memorias --dry-run
```

```
Dry run — files that would be included:
  minhas-memorias/jantar-familia.jpg (Moment)
  minhas-memorias/jardim.jpg (Moment)
  minhas-memorias/reflexao.txt (Reflection)
```

## 4. Criar uma tessera

```bash
tes create minhas-memorias --tags "familia,domingo" --location "Casa"
```

```
Created tessera: 9y2m4a1b3e7d8f0cabc1
```

A saída é o hash de conteúdo em codificação base32. Você pode usá-lo (ou um prefixo curto) nos próximos passos.

## 5. Listar suas tesseras

```bash
tes ls
```

```
┌────────────┬────────────┬──────────┬────────┬────────────┐
│ Hash       │ Created    │ Memories │ Size   │ Visibility │
├────────────┼────────────┼──────────┼────────┼────────────┤
│ 9y2m4a1b3e │ 2026-02-14 │        3 │ 284 KB │ public     │
└────────────┴────────────┴──────────┴────────┴────────────┘
```

## 6. Verificar integridade

Use o hash (ou um prefixo curto) para verificar que todos os arquivos estão intactos e a assinatura é válida:

```bash
tes verify 9y2m4a
```

```
Tessera: 9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
Signature: VALID
  [OK] memories/a1b2c3/media.jpg
  [OK] memories/d4e5f6/media.jpg
  [OK] memories/g7h8i9/media.txt
Verification: PASSED
```

## 7. Exportar uma cópia autocontida

Exporte a tessera para um diretório que pode ser lido sem o Tesseras:

```bash
tes export 9y2m4a ./backup
```

```
Exported to ./backup/tessera-9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
```

O diretório exportado é totalmente autocontido:

```
tessera-9y2m4a1b.../
├── MANIFEST                    # Índice em texto puro com checksums
├── README.decode               # Como ler esta tessera sem software
├── identity/
│   ├── creator.pub.ed25519     # Sua chave pública
│   └── signature.ed25519.sig   # Assinatura do MANIFEST
├── memories/
│   ├── <hash>/
│   │   ├── media.jpg           # A foto
│   │   ├── context.txt         # Descrição em texto puro
│   │   └── meta.json           # Metadados estruturados
│   └── .../
├── schema/
│   └── v1.json                 # Esquema JSON para validação de metadados
└── decode/
    ├── formats.txt             # Explicação de todos os formatos usados
    ├── jpeg.txt                # Como decodificar JPEG
    └── json.txt                # Como decodificar JSON
```

Tudo que um leitor futuro precisa para entender o conteúdo está incluído no próprio diretório — nenhum software Tesseras é necessário.

## 8. Publicar na rede

Com o daemon rodando, publique sua tessera para replicação na rede P2P:

```bash
tes publish 9y2m4a
```

```
Published tessera 9y2m4a1b (24 fragments created)
Distribution in progress — use `tes status 9y2m4a1b` to track.
```

## 9. Verificar status de replicação

Monitore como sua tessera está sendo distribuída:

```bash
tes status 9y2m4a
```

```
Tessera:     9y2m4a1b3e7d8f0cabc123def456789012345678abcdef0123456789abcdef
State:       Healthy
Fragments:   24/24 placed
Peers:       0 holding copies
```

## Opções globais

Estas flags funcionam com qualquer comando:

| Opção | Descrição |
|-------|-----------|
| `-v, --verbose` | Saída detalhada (`-vv` para muito detalhada) |
| `-q, --quiet` | Suprimir todas as mensagens de log |
| `--color <VALOR>` | Coloração: `auto`, `always`, `never` |
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados |
| `--socket <CAMINHO>` | Caminho para o socket Unix do daemon (para `publish`, `fetch`, `status`) |
