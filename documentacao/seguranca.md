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
| 8 | LG | Na primeira instalação a chave da TV vem por HTTP: quem se passar pela TV nesse momento recebe o pacote, com a chave TMDB (risco 7) dentro. Depois, a host key salva (#9) barra o impostor | Média, só na primeira vez |
| 9 | LG, Samsung | Um caractere U+2028/U+2029 num valor do `config.properties` (a cidade, por exemplo) deixa o `config.js` inválido nas TVs antigas (JavaScript antes do ES2019: webOS 3/4, Tizen 3/4). Só acontece se alguém digitá-lo | Baixa |
| 10 | Samsung | Um aparelho que responda na porta 26101 no lugar da TV podia fazer a CLI alocar até 4 GB (tamanho do pacote vindo do cabeçalho), quebrá-la com `max_data` menor que 8 ou mandar saída sem fim | Média (corrigido na CLI em Rust, issue #28) |
| 11 | LG | A resposta do Key Server era lida sem limite de tamanho | Baixa (corrigido na CLI em Rust, issue #28) |

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
3. Fazer a primeira instalação numa rede de confiança (risco 8): é a única vez em que a TV não é conferida.
4. Usar `freemytv webos-install` em vez de `webos/install.py`: ele já verifica a host key da TV (salva na primeira conexão e conferida nas seguintes) e guarda a
   chave com permissão 0600. Veja [rust.md](rust.md).

## Revisão da CLI em Rust (24/10/2026, issue #28)

Revisão de lógica e segurança do código novo do fork (`rust/`):
- **Corrigido:** limites para o que a TV, ou quem se passe por ela, pode fazer a CLI alocar. No SDB: pacote de no
  máximo 1 MB, saída de no máximo 16 MB por comando, e `max_data` de pelo menos 9 bytes. No Key Server da LG:
  resposta de no máximo 64 KB. Os testes simulam um impostor (`refuses_what_an_impostor_could_send`).
- **Corrigido (lógica):** `webos-install` avisa quando o `config.properties` é mais novo que o `FreeMyTV-webos.ipk`
  que vai instalar. O `install.py` instala o pacote antigo sem avisar.
- **Conferido sem problema:** o arquivo temporário da chave é criado com `O_EXCL` e permissão 0600, então não segue
  links simbólicos. O `adb` e o `ssh` recebem argumentos, nunca uma linha de shell montada com texto do usuário. O
  nome das fotos vai codificado na URI. O IP passa por validação antes de qualquer conexão.
- **Fica documentado:** riscos 8 e 9 acima. O 9 vale igual para o Python, e corrigi-lo só no Rust quebraria a
  paridade dos pacotes.
