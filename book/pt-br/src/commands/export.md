# tes export

Exportar uma tessera como um diretório autocontido.

## Uso

```bash
tes export <HASH> <DESTINO>
```

## Argumentos

| Argumento | Descrição |
|-----------|-----------|
| `<HASH>` | Hash de conteúdo da tessera (64 caracteres hexadecimais) |
| `<DESTINO>` | Diretório de destino |

## Opções

| Opção | Descrição |
|-------|-----------|
| `--data-dir <CAMINHO>` | Diretório base para armazenamento de dados (padrão: `~/.tesseras`) |

## Estrutura de saída

A exportação cria um diretório chamado `tessera-<hash>` dentro do destino:

```
tessera-9f2c4a1b.../
├── MANIFEST                    # Índice em texto puro com checksums
├── README.decode               # Instruções de decodificação legíveis por humanos
├── identity/
│   ├── creator.pub.ed25519     # Chave pública do criador
│   └── signature.ed25519.sig   # Assinatura do MANIFEST
├── memories/
│   ├── <hash-conteudo>/
│   │   ├── media.jpg           # Arquivo de mídia principal
│   │   ├── context.txt         # Contexto humano em UTF-8 puro
│   │   └── meta.json           # Metadados estruturados
│   └── .../
├── schema/
│   └── v1.json                 # Esquema JSON para validação de metadados
└── decode/
    ├── formats.txt             # Explicação de todos os formatos usados
    ├── jpeg.txt                # Como decodificar JPEG
    ├── wav.txt                 # Como decodificar WAV
    └── json.txt                # Como decodificar JSON
```

## Exemplo

```bash
tes export 9f2c4a1b3e7d8f0cabc123def4567890... ./backup
```

```
Exported to ./backup/tessera-9f2c4a1b3e7d8f0cabc123def4567890...
```

## Característica principal: autocontido

O diretório exportado é projetado para ser legível **sem o software Tesseras**. Ele inclui:

- **MANIFEST** — um arquivo em texto puro listando cada arquivo com seu checksum BLAKE3, legível por qualquer editor de texto
- **README.decode** — instruções legíveis por humanos para compreender o conteúdo
- **decode/** — explicações detalhadas de cada formato de arquivo usado (JPEG, WAV, JSON, UTF-8)

Isso significa que alguém daqui a milhares de anos, sem conhecimento algum sobre o Tesseras, ainda pode entender e acessar as memórias.

## Casos de uso

- **Backup** — exporte para um disco externo, pendrive ou armazenamento em nuvem
- **Compartilhamento** — entregue a alguém uma cópia completa de uma tessera
- **Arquivamento** — armazene em mídia de escrita única (DVD, Blu-ray, fita)
- **Migração** — mova tesseras entre máquinas sem precisar do banco de dados
