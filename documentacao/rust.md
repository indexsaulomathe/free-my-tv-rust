# Conversão para Rust

**Meta:** converter o projeto inteiro para Rust, item por item, sempre com testes de paridade e benchmark.
**Regra:** cada item tem uma issue antes de ser convertido (pasta [`issues/`](issues/)) e cada mudança é registrada
em [`alteracoes.md`](alteracoes.md).

## Fases

| Fase | O que | Status |
|---|---|---|
| 1. Ferramentas | Scripts Python de build, instalação e geração | Em andamento: 12 de 13 itens feitos |
| 2. Lógica | Dados, formatação e regras (`web/data.js`, `HomeData`, `Recs`, `HomeMode`) → Rust compilado para WASM e JNI | Em andamento: web feito (#14), Android a fazer (#15) |
| 3. Interface | `home.js`, `HomeView` e afins | A avaliar: a interface depende do motor da TV, e o ganho precisa ser medido antes |

### Limites conhecidos
- **WebAssembly exige Chromium 57+.** webOS 3/4 e Tizen 3/4 (TVs de 2017 a 2019) não rodam WASM. Nessas TVs a
  versão JS continua sendo usada como fallback.
- **Android:** a tela inicial, o protetor de tela e os serviços dependem de APIs Java. O Rust entra como biblioteca
  nativa (NDK/JNI), e uma casca Java continua existindo.
- **Assinatura Samsung** continua com o CLI `tizen` da Samsung.

## Estrutura

```
rust/
├── Cargo.toml                    workspace
└── crates/
    ├── freemytv-core/            biblioteca: config, i18n, webapp, package, homebrew, zones, json, instaladores,
    │                             traduções, fotos, empacotamento do WASM
    ├── freemytv-logic/           a lógica pura do app web, compilada para WebAssembly (fase 2)
    └── freemytv-cli/             o comando `freemytv`
        └── examples/peak_rss.rs  mede o pico de memória para o benchmark
```

## Itens

| # | Issue | Python | Rust | Status |
|---|---|---|---|---|
| 01 | [config](issues/01-config.md) | `webapp.read_config` | `config.rs` | ✅ |
| 02 | [JSON igual ao Python](issues/02-json-python.md) | `json.dumps` | `json.rs` | ✅ |
| 03 | [catálogos i18n](issues/03-i18n-catalogos.md) | `translate.web_keys`, `webapp.catalogues` | `i18n.rs` | ✅ |
| 04 | [arquivos do app web](issues/04-webapp-files.md) | `webapp.files` | `webapp.rs` | ✅ |
| 05 | [pacote LG](issues/05-webos-build.md) | `webos/build.py` | `freemytv webos-build` | ✅ |
| 06 | [pacote Samsung](issues/06-tizen-build.md) | `tizen/build.py` | `freemytv tizen-build` | ✅ (sem assinatura) |
| 07 | [Homebrew](issues/07-homebrew-repo.md) | `tools/homebrew_repo.py` | `freemytv homebrew-repo` | ✅ |
| 08 | [fusos](issues/08-make-zones.md) | `tools/make_zones.py` | `freemytv make-zones` | ✅ |
| 09 | [instalador LG](issues/09-webos-install.md) | `webos/install.py` | `freemytv webos-install` | ✅ (com a correção da host key) |
| 10 | [instalador Samsung](issues/10-tizen-install.md) | `tizen/install.py` | `freemytv tizen-install` | ✅ (testado com uma TV simulada) |
| 11 | [validação de traduções](issues/11-translate-check.md) | `translate.py --check` | `freemytv translate-check` | ✅ |
| 12 | [fotos](issues/12-photos.md) | `tools/photos.py` | `freemytv photos` | ✅ (testado com um adb simulado) |
| 13 | [arte e símbolos](issues/13-arte.md) | `make_art.py`, `make_symbols.py` | — | ⏳ baixa prioridade |
| 14 | [lógica web](issues/14-logica-web.md) | `web/data.js`, `strings.js` | `freemytv-logic` (WASM) | ✅ |
| 15 | [lógica Android](issues/15-logica-android.md) | `HomeData`, `Recs`, `HomeMode` | JNI | ⏳ fase 2 |
| 16 | [interface web](issues/16-ui-web.md) | `home.js` | — | ⏳ fase 3 |
| 17 | [interface Android](issues/17-ui-android.md) | `HomeView` etc. | — | ⏳ fase 3 |

## Paridade

Os arquivos gerados pelo Rust são **idênticos** aos do Python (verificado por `tests/python/test_rust_parity.py`):
- `.ipk`: mesmos membros, mesmo `control`, mesmos arquivos, modos e ordem (só a data dos arquivos muda);
- `.wgt`: mesmos nomes, mesma ordem e mesmo conteúdo;
- `apps.json` e `zones.js`: idênticos byte a byte.

## Como usar

```bash
cd rust && cargo build --release            # gera rust/target/release/freemytv
cd ..
rust/target/release/freemytv webos-build --generic          # = python3 webos/build.py --generic
rust/target/release/freemytv tizen-build                    # = python3 tizen/build.py
rust/target/release/freemytv homebrew-repo FreeMyTV-webos.ipk
rust/target/release/freemytv make-zones
rust/target/release/freemytv webos-install 192.168.1.50   # = python3 webos/install.py (--uninstall remove)
rust/target/release/freemytv tizen-install 192.168.1.50   # = python3 tizen/install.py (--uninstall remove)
rust/target/release/freemytv translate-check              # = python3 tools/translate.py --check
rust/target/release/freemytv photos 192.168.1.50 Ferias/  # = python3 tools/photos.py (TVs Android, precisa do adb)
```

`webos-install` usa a mesma chave do Python (`~/.ssh/freemytv_webos_<IP>`), então quem já instalou pelo Python não
digita a passphrase de novo. Diferenças: a host key da TV fica em `~/.ssh/freemytv_webos_known_hosts` na primeira
conexão e é conferida nas seguintes, e a passphrase é pedida pelo `ssh-keygen` (não aparece na linha de comando).
Se o Developer Mode for reinstalado, a TV troca de host key e a instalação para com a dica do `ssh-keygen -R`.
`tizen-install` fala SDB como o Python (porta 26101) e instala o `.wgt` já assinado (`python3 tizen/build.py --sign`,
a assinatura continua com o CLI `tizen` da Samsung). Foi testado contra uma TV simulada (`tizen_install.rs`), não
numa Samsung de verdade.

## Benchmark (fase 1)

`python3 tests/bench/bench.py --runs 30 --markdown`. Cada comando roda como processo novo, contando a
partida do interpretador, que é o tempo que a pessoa espera. A memória é o pico de RSS do processo.

Ambiente: servidor Linux x86_64, 2 CPUs, Python 3.13.17, Rust 1.97 (`--release`, LTO). **Não é a TV:** essas
ferramentas rodam no computador, nunca na TV. 30 execuções por comando, mediana.

| Ferramenta | Python | Rust | Ganho de tempo | Memória Python | Memória Rust |
|---|---:|---:|---:|---:|---:|
| Pacote LG `.ipk` (`--generic`) | 172,4 ms ± 11,4 | 68,0 ms ± 7,0 | 2,5× | 22,2 MB | 7,4 MB |
| Pacote Samsung `.wgt` (`--generic`) | 130,4 ms ± 24,3 | 31,2 ms ± 3,0 | 4,2× | 22,6 MB | 6,9 MB |
| Homebrew `apps.json` | 22,3 ms ± 1,3 | 2,5 ms ± 0,2 | 9,0× | 14,4 MB | 4,1 MB |
| Fusos `zones.js` | 24,3 ms ± 2,3 | 3,1 ms ± 0,2 | 7,9× | 12,8 MB | 3,6 MB |
| Traduções `--check`¹ | 108,6 ms ± 10,8 | 11,9 ms ± 0,5 | 9,2× | 24,2 MB | 6,9 MB |

¹ Medido depois, noutra máquina (Linux x86_64, Python 3, Rust 1.98, 15 execuções). A primeira versão em Rust era
mais lenta que o Python (111 ms), porque recompilava as regex a cada chamada; agora elas são compiladas uma vez.

**Leitura:**
- Nos comandos pequenos, quase todo o ganho vem de não precisar iniciar o Python (cerca de 20 ms).
- No `.ipk`, o tempo do Rust é dominado pela compressão gzip no nível máximo (o mesmo nível do Python). A diferença
  restante vem da montagem dos arquivos e da extração das traduções.
- A memória cai de 3 a 4 vezes.
- Na prática, ninguém percebe 100 ms num build. O ganho real da fase 1 é **um binário único sem dependências**
  (não precisa de Python), arquivos idênticos e a base de código para as fases 2 e 3.

## Fase 2: a lógica do app web em WebAssembly (issue #14)

**O que foi para o Rust** (`rust/crates/freemytv-logic`): as funções puras de `web/data.js` e `web/strings.js`, que
não dependem do navegador:

| JavaScript | Rust | O que faz |
|---|---|---|
| `parseWeather`, `localTime` | `parse_weather`, `local_time` | lê a previsão do Open-Meteo |
| `Data.condition`, `Data.symbol` | `condition`, `symbol` | palavras e ícone do tempo pelo código WMO |
| `franchise`, `service`, `bestBackdrop` | `franchise`, `service`, `best_backdrop` | regras das sugestões do TMDB |
| `pluralOf` (sem Intl), escolha da forma em `plural` | `plural_category`, `plural_form` | plurais CLDR |
| `base`, `localeOf`, `resolve` | `base`, `locale_of`, `resolve` | escolha do idioma e do locale |

O que continua em JavaScript: rede (XHR), localStorage, `Intl` (nomes de meses, AM/PM, números), o desenho da tela
(`home.js`) e o acesso à TV (`tv.js`).

**Como funciona:**
- `freemytv make-logic` compila o crate para `wasm32-unknown-unknown`, tira as seções de depuração e grava
  `web/logic_wasm.js` (o WASM em base64, carregado com os outros scripts, sem XHR: funciona em `file://`). O arquivo
  gerado vai no repositório, como o `zones.js`, então o build da TV não precisa de Rust.
- `web/logic.js` carrega o módulo (assíncrono) e expõe `Logic`. Cada função do `data.js`/`strings.js` pergunta ao
  `Logic` primeiro; enquanto o módulo carrega, nas TVs sem WebAssembly (webOS 3/4, Tizen 3/4) e para entradas que o
  JavaScript converteria ou em que daria erro, `Logic` responde `undefined` e o JavaScript faz a conta.
- Testado na TV LG (webOS 10.3, Chrome 120): `Logic.ready()` verdadeiro e as respostas certas em português.

**Paridade:** a suíte web inteira (29 testes) roda duas vezes, em JavaScript e com o WASM (`FMTV_WASM=1`), e
`tests/web/logic.test.js` compara as duas versões em centenas de entradas, inclusive estranhas (NaN, tipos errados,
listas vazias, datas fora do intervalo): tudo igual.

**Desempenho (medido, `node tests/bench/logic_bench.js`, V8 do Node 26):**

| Chamada | JavaScript | Rust (WASM) | Rust / JS |
|---|---:|---:|---:|
| `Data.condition` | 217 ns | 627 ns | 2,9× |
| `Data.symbol` | 78 ns | 468 ns | 6,0× |
| `I18N.plural` (ru, sem Intl) | 1,8 µs | 7,3 µs | 4,1× |
| `parseWeather` (48 h) | 9,4 µs | 31,5 µs | 3,4× |
| `franchise` | 154 ns | 1,8 µs | 11,4× |

**Leitura honesta:** o WASM é **de 3 a 11 vezes mais lento** que o JavaScript nessas funções. Elas são pequenas,
e passar textos entre JS e WASM (codificar em UTF-8, copiar, decodificar, e `JSON.stringify`/`parse` no clima) custa
mais que a conta. Na prática ninguém percebe: o pior caso são 31 µs a cada 30 minutos, e um quadro tem 16 ms. O
pacote cresce 90 KB (`.ipk` de 663 para 753 KB) e o módulo reserva cerca de 1 MB de memória na TV. O ganho é ter
uma única fonte da lógica em Rust, testada, que o Android (#15, JNI) pode usar também.
