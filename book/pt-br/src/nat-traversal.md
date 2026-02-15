# Travessia de NAT

A maioria dos dispositivos na internet ficam atras de um **NAT** (Network Address Translator). Seu roteador atribui ao seu dispositivo um endereco privado (como `192.168.1.100`) e o traduz para um endereco publico quando voce conecta para fora. Isso funciona bem para navegar na web, mas cria um problema para redes P2P: dois dispositivos atras de NATs diferentes nao conseguem se conectar diretamente sem ajuda.

Tesseras resolve isso com uma abordagem em tres camadas, tentando a opcao mais barata primeiro:

1. **Conexao direta** — se ambos os nos tem IPs publicos, eles conectam diretamente
2. **UDP hole punching** — um terceiro no apresenta os dois peers para que eles possam furar seus NATs
3. **Relay** — um no com IP publico encaminha pacotes entre os dois peers

## Descoberta do tipo de NAT

Quando um no inicia, ele envia requisicoes STUN (Session Traversal Utilities for NAT) para multiplos servidores publicos. Comparando os enderecos externos que esses servidores reportam, o no classifica seu NAT:

| Tipo de NAT | O que significa | Hole punching? |
|-------------|-----------------|----------------|
| **Public** | Sem NAT — seu dispositivo tem IP publico | Nao necessario |
| **Cone** | NAT mapeia a mesma porta interna para a mesma porta externa independente do destino | Funciona bem (~80%) |
| **Symmetric** | NAT atribui uma porta externa diferente para cada destino | Nao confiavel |
| **Unknown** | Nao conseguiu alcancar servidores STUN | Relay necessario |

Seu no divulga seu tipo de NAT em mensagens Pong do DHT, para que outros nos saibam se hole punching vale a pena tentar.

## Hole punching

Quando o no A (atras de um NAT Cone) quer conectar ao no B (tambem atras de um NAT Cone), nenhum consegue alcancar o outro diretamente. A solucao:

1. A envia uma mensagem **PunchIntro** ao no I (um introdutor — qualquer no com IP publico que ambos conhecam). A mensagem inclui o endereco externo de A (do STUN) e uma assinatura Ed25519 provando a identidade de A.

2. I verifica a assinatura e encaminha um **PunchRequest** a B, incluindo o endereco de A e a assinatura original.

3. B verifica a assinatura (provando que a requisicao realmente veio de A, nao de uma fonte falsificada). B entao envia um pacote UDP para o endereco externo de A — isso abre um pinhole no NAT de B. B tambem envia uma mensagem **PunchReady** de volta a A com o endereco externo de B.

4. A envia um pacote UDP para o endereco externo de B. Ambos os NATs agora tem pinholes, e os dois nos podem se comunicar diretamente.

O processo inteiro leva 2-5 segundos. As assinaturas Ed25519 previnem **ataques de reflexao**, onde um atacante reproduz uma introducao antiga para redirecionar trafego.

## Fallback por relay

Quando hole punching falha (NAT Symmetric, firewalls estritos ou redes corporativas), nos usam relay atraves de um no com IP publico:

1. A envia um **RelayRequest** ao no R (um no com IP publico com relay habilitado).
2. R cria uma sessao e envia um **RelayOffer** a ambos A e B, contendo o endereco do relay e um token de sessao.
3. A e B enviam seus pacotes a R, prefixados com o token de sessao. R remove o token e encaminha o payload ao outro peer.

Sessoes de relay tem limites de largura de banda:
- **256 KB/s** para peers com boa reciprocidade (eles armazenam fragmentos para outros)
- **64 KB/s** para peers sem reciprocidade
- Sessoes nao reciprocas sao limitadas a 10 minutos

Isso incentiva nos a contribuir armazenamento — bons cidadaos da rede recebem melhor servico de relay.

## Migracao de endereco

Quando um dispositivo movel troca de rede (Wi-Fi para celular), seu endereco IP muda. Ao inves de encerrar e reconstruir sessoes de relay, o no envia uma mensagem **RelayMigrate** assinada para atualizar seu endereco na sessao existente. Isso evita reestabelecer conexoes do zero.

## Configuracao

A secao `[nat]` na configuracao do daemon controla a travessia de NAT:

```toml
[nat]
# Servidores STUN para deteccao de tipo de NAT
stun_servers = ["stun.l.google.com:19302", "stun.cloudflare.com:3478"]

# Habilitar relay (encaminhar trafego para outros nos com NAT)
relay_enabled = false

# Maximo de sessoes de relay simultaneas
relay_max_sessions = 50

# Limite de largura de banda para peers reciprocos (KB/s)
relay_reciprocal_kbps = 256

# Limite de largura de banda para peers nao reciprocos (KB/s)
relay_bootstrap_kbps = 64

# Timeout de inatividade de sessao relay (segundos)
relay_idle_timeout_secs = 60
```

Para executar um no relay, defina `relay_enabled = true`. Seu no deve ter um IP publico (ou roteador com port forwarding) para servir como relay.

## Reconexao mobile

Quando o app Tesseras detecta uma mudanca de rede em um dispositivo movel, ele executa uma sequencia de reconexao em tres fases:

1. **Migracao QUIC** (0-2s) — QUIC suporta migracao de conexao nativamente. O app tenta migrar todas as conexoes ativas para o novo endereco.
2. **Re-STUN** (2-5s) — descobre o novo endereco externo e re-anuncia ao DHT.
3. **Reestabelecimento** (5-10s) — reconecta peers que a migracao nao conseguiu salvar, em ordem de prioridade: nos bootstrap primeiro, depois nos que guardam seus fragmentos, depois nos cujos fragmentos voce guarda.

O app mostra progresso de reconexao atraves do stream de eventos `NetworkChanged`.

## Monitoramento

A travessia de NAT expoe metricas Prometheus em `/metrics`:

- `tesseras_nat_type` — tipo de NAT detectado atualmente
- `tesseras_stun_requests_total` / `tesseras_stun_failures_total` — confiabilidade STUN
- `tesseras_punch_attempts_total{initiator_nat, target_nat}` — taxa de sucesso de punch por par de NAT
- `tesseras_relay_sessions_active` — carga atual de relay
- `tesseras_relay_bytes_forwarded` — largura de banda total de relay
- `tesseras_network_change_total` — frequencia de mudanca de rede no mobile
