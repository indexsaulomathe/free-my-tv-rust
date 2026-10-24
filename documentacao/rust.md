# Conversão para Rust

**Meta:** converter o projeto inteiro para Rust, item por item, sempre com testes de paridade e benchmark.
**Regra:** cada item tem uma issue antes de ser convertido (pasta [`issues/`](issues/)) e cada mudança é registrada
em [`alteracoes.md`](alteracoes.md).

## Fases

| Fase | O que | Status |
|---|---|---|
| 1. Ferramentas | Scripts Python de build, instalação e geração | Em andamento: 8 de 13 itens feitos |
| 2. Lógica | Dados, formatação e regras (`web/data.js`, `HomeData`, `Recs`, `HomeMode`) → Rust compilado para WASM e JNI | A fazer |
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
    ├── freemytv-core/            biblioteca: config, i18n, webapp, package, homebrew, zones, json
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
| 09 | [instalador LG](issues/09-webos-install.md) | `webos/install.py` | `freemytv webos-install` | ⏳ próximo, com a correção da host key |
| 10 | [instalador Samsung](issues/10-tizen-install.md) | `tizen/install.py` | `freemytv tizen-install` | ⏳ |
| 11 | [validação de traduções](issues/11-translate-check.md) | `translate.py --check` | `freemytv translate-check` | ⏳ |
| 12 | [fotos](issues/12-photos.md) | `tools/photos.py` | `freemytv photos` | ⏳ |
| 13 | [arte e símbolos](issues/13-arte.md) | `make_art.py`, `make_symbols.py` | — | ⏳ baixa prioridade |
| 14 | [lógica web](issues/14-logica-web.md) | `web/data.js`, `strings.js` | WASM | ⏳ fase 2 |
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
```

A instalação na TV continua sendo feita por `python3 webos/install.py <IP>` até a issue 09 ser concluída.

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

**Leitura:**
- Nos comandos pequenos, quase todo o ganho vem de não precisar iniciar o Python (cerca de 20 ms).
- No `.ipk`, o tempo do Rust é dominado pela compressão gzip no nível máximo (o mesmo nível do Python). A diferença
  restante vem da montagem dos arquivos e da extração das traduções.
- A memória cai de 3 a 4 vezes.
- Na prática, ninguém percebe 100 ms num build. O ganho real da fase 1 é **um binário único sem dependências**
  (não precisa de Python), arquivos idênticos e a base de código para as fases 2 e 3.
