# tes export

Exportar uma tessera como um diretório autocontido.

Alias: `tes e`

## Uso

```bash
tes export <HASH> <DESTINO>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash da tessera ou prefixo (base32 ou hex) |
| `<DESTINO>` | Diretório de destino |

Você pode usar o hash completo ou um prefixo curto. Ambos os formatos base32 e hex são aceitos.

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.local/share/tesseras`) |

## Exemplos

### Exportar para um diretório de backup

```bash
tes export 9y2m4a ./backup
```

```
Exported to ./backup/tessera-9y2m4a1b3e7d8f0cabc123def456789012345678abcdef
```

### Exportar para pendrive

```bash
tes export 9y2m4a /mnt/usb/tesseras
```

## Estrutura do diretório exportado

O diretório exportado é totalmente autocontido — legível sem o software Tesseras:

```
tessera-9y2m4a1b.../
├── MANIFEST                    # Índice em texto puro com checksums
├── README.decode               # Como ler esta tessera sem software
├── identity/
│   ├── creator.pub.ed25519     # Chave pública do criador
│   └── signature.ed25519.sig   # Assinatura do MANIFEST
├── memories/
│   ├── <hash>/
│   │   ├── media.jpg           # A foto/áudio/vídeo/texto
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

Tudo que um leitor futuro precisa para entender o conteúdo está incluído — nenhum software Tesseras é necessário.

## Característica principal: autocontido

O diretório exportado é projetado para ser legível **sem o software Tesseras**. Ele inclui:

- **MANIFEST** — um arquivo em texto puro listando cada arquivo com seu checksum BLAKE3, legível por qualquer editor de texto
- **README.decode** — instruções legíveis por humanos para compreender o conteúdo
- **decode/** — explicações detalhadas de cada formato de arquivo usado (JPEG, WAV, JSON, UTF-8)

Isso significa que alguém daqui a milhares de anos, sem conhecimento algum sobre o Tesseras, ainda pode entender e acessar as memórias.

## Casos de uso

- **Backup offline** — copie para pendrives, discos externos ou NAS
- **Mídia de arquivo** — grave em M-DISC, fita ou imprima QR codes
- **Compartilhamento** — envie uma cópia autocontida para alguém sem o Tesseras
- **Migração** — mova tesseras entre sistemas sem usar a rede
