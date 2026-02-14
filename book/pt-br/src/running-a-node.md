# Executando um No

O binario `tesseras-daemon` executa um no completo do Tesseras que participa da rede peer-to-peer. Ele escuta conexoes sobre QUIC, entra na tabela hash distribuida (DHT) e permite que outros nos descubram e encontrem ponteiros de tesseras.

## Iniciando o daemon

```bash
tesseras-daemon
```

Na primeira execucao, o daemon:

1. Cria o diretorio de dados (`~/.local/share/tesseras` no Linux, `~/Library/Application Support/tesseras` no macOS)
2. Gera uma identidade de no com prova de trabalho (leva cerca de 1 segundo)
3. Abre um listener QUIC em `0.0.0.0:4433`
4. Faz bootstrap na rede contactando nos semente
5. Imprime `daemon ready` quando totalmente operacional

## Opcoes de linha de comando

```
tesseras-daemon [OPTIONS]
```

| Opcao | Descricao | Padrao |
|-------|-----------|--------|
| `-c, --config <PATH>` | Caminho para um arquivo de configuracao TOML | Nenhum (usa padroes internos) |
| `-l, --listen <ADDR>` | Endereco e porta para escutar | `0.0.0.0:4433` |
| `-b, --bootstrap <ADDRS>` | Enderecos de bootstrap separados por virgula | `boot1.tesseras.net:4433,boot2.tesseras.net:4433` |
| `-d, --data-dir <PATH>` | Diretorio de dados | Especifico da plataforma (veja acima) |

Opcoes CLI sobrescrevem valores do arquivo de configuracao.

## Exemplos

Executar com padroes (entrar na rede publica):

```bash
tesseras-daemon
```

Executar como no semente (sem bootstrap, outros nos conectam a voce):

```bash
tesseras-daemon --bootstrap ""
```

Executar em uma porta personalizada com um diretorio de dados especifico:

```bash
tesseras-daemon --listen 0.0.0.0:5000 --data-dir /var/lib/tesseras
```

Fazer bootstrap a partir de um no especifico:

```bash
tesseras-daemon --bootstrap "192.168.1.50:4433"
```

Entrar em uma rede local com multiplos nos:

```bash
tesseras-daemon --bootstrap "192.168.1.10:4433,192.168.1.11:4433"
```

## Identidade do no

Cada no tem uma identidade unica armazenada em `<data-dir>/identity.key`. Este arquivo contem uma chave publica de 32 bytes e um nonce de prova de trabalho de 8 bytes.

O ID do no e derivado da chave publica: `BLAKE3(pubkey || nonce)` truncado para 20 bytes. O nonce deve produzir um hash com 8 bits zero iniciais, o que leva cerca de 256 tentativas de hash. Esta prova de trabalho leve torna caro criar milhares de identidades falsas enquanto custa menos de um segundo para usuarios legitimos.

A identidade e gerada automaticamente na primeira execucao e reutilizada nas execucoes seguintes. Se voce apagar `identity.key`, uma nova identidade sera gerada.

## Logging

O daemon usa logging estruturado via `tracing`. Controle o nivel de log com a variavel de ambiente `RUST_LOG`:

```bash
# Padrao (nivel info)
tesseras-daemon

# Logging de debug
RUST_LOG=debug tesseras-daemon

# Mostrar apenas avisos e erros
RUST_LOG=warn tesseras-daemon

# Debug para DHT, info para o resto
RUST_LOG=info,tesseras_dht=debug tesseras-daemon
```

## Desligamento

Pressione **Ctrl+C** para iniciar o desligamento gracioso. O daemon ira:

1. Parar de aceitar novas conexoes
2. Finalizar operacoes em andamento (ate 5 segundos)
3. Fechar todas as conexoes QUIC
4. Sair de forma limpa

## Firewall

O daemon se comunica pela porta UDP 4433 (QUIC). Se voce esta atras de um firewall, certifique-se de que esta porta esta aberta para trafego UDP de entrada e saida.

```bash
# Exemplo: Linux com ufw
sudo ufw allow 4433/udp
```
