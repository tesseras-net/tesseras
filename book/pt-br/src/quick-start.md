# Início Rápido

Este tutorial guia você por um fluxo completo: criar uma identidade, construir uma tessera a partir de arquivos, verificá-la e exportá-la.

## 1. Inicializar sua identidade

Primeiro, configure sua identidade local e banco de dados:

```bash
tes init
```

```
Generated Ed25519 identity
Database initialized
Config written to /home/user/.tesseras/config.toml
Tesseras initialized at /home/user/.tesseras
```

Isso cria:

- `~/.tesseras/identity/` — seu par de chaves Ed25519
- `~/.tesseras/db/` — banco de dados SQLite para indexação
- `~/.tesseras/blobs/` — armazenamento para arquivos de memória
- `~/.tesseras/config.toml` — arquivo de configuração

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

## 4. Criar uma tessera

```bash
tes create minhas-memorias --tags "familia,domingo" --location "Casa"
```

A saída inclui o hash de conteúdo — uma string hexadecimal de 64 caracteres que identifica unicamente sua tessera. Copie-o para os próximos passos.

## 5. Listar suas tesseras

```bash
tes list
```

```
Hash             Created     Memories  Size    Visibility
9f2c4a1b3e7d8f0c 2026-02-14         3  284 KB  public
```

## 6. Verificar integridade

Use o hash de conteúdo para verificar que todos os arquivos estão intactos e a assinatura é válida:

```bash
tes verify 9f2c4a1b3e7d8f0c...
```

```
Tessera: 9f2c4a1b3e7d8f0c...
Signature: VALID
  [OK] memories/a1b2c3/media.jpg
  [OK] memories/d4e5f6/media.jpg
  [OK] memories/g7h8i9/media.txt
Verification: PASSED
```

## 7. Exportar uma cópia autocontida

Exporte a tessera para um diretório que pode ser lido sem o Tesseras:

```bash
tes export 9f2c4a1b3e7d8f0c... ./backup
```

```
Exported to ./backup/tessera-9f2c4a1b3e7d8f0c...
```

## 8. Inspecionar a exportação

O diretório exportado é totalmente autocontido:

```
tessera-9f2c4a1b3e7d8f0c.../
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
└── decode/
    ├── formats.txt             # Explicação de todos os formatos usados
    ├── jpeg.txt                # Como decodificar JPEG
    └── json.txt                # Como decodificar JSON
```

Tudo que um leitor futuro precisa para entender o conteúdo está incluído no próprio diretório — nenhum software Tesseras é necessário.
