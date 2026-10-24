# Análise de segurança

Revisão do código em 24/10/2026, no commit `aa34335` (igual ao upstream).

**Resumo:** nenhum indício de código malicioso. Não há telemetria, ofuscação, `eval` nem download de código em
tempo de execução. Os riscos vêm do próprio funcionamento do app, que precisa de acesso de desenvolvedor à TV.

## Comunicação externa

Só HTTPS, e o Android bloqueia HTTP em texto claro (`usesCleartextTraffic=false`):
`api.open-meteo.com`, `api.themoviedb.org` e `image.tmdb.org`, `api.binance.com` e `api.binance.us`, e Watchmode
(opcional).

## Riscos

| # | Plataforma | Risco | Gravidade |
|---|---|---|---|
| 1 | Android | O app exige a depuração ADB ligada o tempo todo (porta 5555 aberta para a rede local) e guarda uma chave ADB com poder de shell | Alta, se a rede não for confiável |
| 2 | Android | Desativa apps da lista `DEBLOAT` com `pm disable-user`. Nada é desinstalado, e o modo Original reativa tudo | Baixa |
| 3 | Android | Usa um serviço de acessibilidade, que só filtra teclas (`flagRequestFilterKeyEvents`) e não lê a tela | Baixa |
| 4 | LG | `webos/install.py` usa `StrictHostKeyChecking=no` e busca a chave por HTTP (porta 9991). Alguém na rede pode se passar pela TV | Média (corrigido na CLI em Rust, issue #9) |
| 5 | LG | A chave SSH fica sem senha em `~/.ssh/freemytv_webos_<IP>` | Média |
| 6 | LG | Com a autorização na TV, o app ganha controle pelo WebSocket `127.0.0.1:3000` | Baixa |
| 7 | Todas | A chave TMDB fica embutida no pacote (`config.js`) | Baixa |

## Pontos positivos

- `PhotoInbox` só aceita escrita do UID do shell ADB (2000) ou do root e bloqueia `/`, `..` e nomes ocultos.
- Os comandos de shell usam nomes de pacote do sistema, nunca texto digitado pelo usuário.
- No JS, `innerHTML` só é usado para limpar elementos (`= ''`), sem injetar HTML.
- O `Watchdog` e o modo Original garantem um caminho de volta.
- O `apps.json` do Homebrew traz o SHA-256 e o tamanho do pacote.
- `CLAUDE.md` e `AGENTS.md` não trazem instruções perigosas e mandam nunca expor chaves.

## Recomendações

1. Desligar o **Key Server** na TV depois de instalar.
2. Deixar a TV numa rede confiável, de preferência separada da rede de visitantes.
3. Usar `freemytv webos-install` em vez de `webos/install.py`: ele já verifica a host key da TV (salva na primeira conexão e conferida nas seguintes) e guarda a
   chave com permissão 0600. Veja [rust.md](rust.md).
