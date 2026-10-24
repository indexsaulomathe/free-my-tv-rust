# Conversão para Rust

**Meta:** converter o projeto inteiro para Rust, item por item, sempre com testes de paridade e benchmark.
**Regra:** cada item tem uma issue antes de ser convertido (pasta [`issues/`](issues/)) e cada mudança é registrada
em [`alteracoes.md`](alteracoes.md).

## Fases

| Fase | O que | Status |
|---|---|---|
| 1. Ferramentas | Scripts Python de build, instalação e geração | Concluída: 12 itens em Rust; arte e símbolos ficam em Python (#13, regressão de fidelidade) |
| 2. Lógica | Dados, formatação e regras (`web/data.js`, `HomeData`, `Recs`, `HomeMode`) → Rust compilado para WASM e JNI | Avaliada: regressão na web (#14, revertida na #22) e no Android (#15). Fica em JS e Java |
| 3. Interface | `home.js`, `HomeView` e afins | Não será feita: a fase 2 regrediu, e a interface depende ainda mais do motor da TV (#16, #17) |

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
    │                             traduções, fotos
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
| 13 | [arte e símbolos](issues/13-arte.md) | `make_art.py`, `make_symbols.py` | protótipo | ❌ regressão: fica em Python |
| 14 | [lógica web](issues/14-logica-web.md) | `web/data.js`, `strings.js` | `freemytv-logic` (WASM) | ↩️ revertido (#22): regressão, fica em JS |
| 15 | [lógica Android](issues/15-logica-android.md) | `HomeData`, `Recs`, `HomeMode` | JNI | ❌ regressão (protótipo medido) |
| 16 | [interface web](issues/16-ui-web.md) | `home.js` | — | ❌ não será feita |
| 17 | [interface Android](issues/17-ui-android.md) | `HomeView` etc. | — | ❌ não será feita |

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

## Probe ARMv7 para webOS (issue #19)

`rust/probes/webos_armv7.rs` é um executável Rust mínimo, sem `std` nem bibliotecas externas. A função Rust retorna
`42`; a entrada usa a syscall Linux ARM EABI `exit`. Compilar no computador:

```bash
rustup target add armv7-unknown-linux-gnueabi
rustc rust/probes/webos_armv7.rs --target armv7-unknown-linux-gnueabi \
  -C panic=abort -C opt-level=s -C linker=rust-lld \
  -C link-arg=-static -C link-arg=-nostdlib -C link-arg=-no-pie \
  -o /tmp/freemytv-webos-armv7-probe
file /tmp/freemytv-webos-armv7-probe
readelf -h -l /tmp/freemytv-webos-armv7-probe
```

ELF esperado: ARM 32-bit little-endian, EABI5 soft-float, estático e sem `INTERP`.
Verificação local em 2026-10-24: `rustc` 1.98.1 gerou ELF32 ARM EABI5 soft-float estático; `readelf` não mostrou
`INTERP`; disassembly mostrou `rust_probe` retornando `42` em `r0` e `_start` chamando `exit` (syscall 1).

**Pedir aprovação antes de qualquer comando remoto.** Após aprovação, copiar apenas para `/tmp`, executar como
`prisoner` e verificar `probe_exit=42`. Defina `TV_IP` e `SSH_KEY` somente no shell local; não registre seus valores
no repositório. Esta TV não tem `scp` remoto, então a cópia usa `cat` sobre SSH:

```bash
ssh -p 9922 -i "$SSH_KEY" \
  -o HostKeyAlgorithms=+ssh-rsa -o PubkeyAcceptedAlgorithms=+ssh-rsa \
  -o StrictHostKeyChecking=yes -o BatchMode=yes prisoner@"$TV_IP" \
  'umask 077; cat > /tmp/freemytv-webos-armv7-probe && chmod 700 /tmp/freemytv-webos-armv7-probe' \
  < /tmp/freemytv-webos-armv7-probe
ssh -p 9922 -i "$SSH_KEY" \
  -o HostKeyAlgorithms=+ssh-rsa -o PubkeyAcceptedAlgorithms=+ssh-rsa \
  -o StrictHostKeyChecking=yes -o BatchMode=yes prisoner@"$TV_IP" \
  '/tmp/freemytv-webos-armv7-probe; printf "probe_exit=%s\n" "$?"'
```

Resultado na TV em 2026-10-24: `probe_exit=42`; arquivo removido e ausência confirmada com `test ! -e`.

Rollback, também após aprovação para comando remoto: usar o mesmo `ssh` acima com
`'rm -f /tmp/freemytv-webos-armv7-probe'`. Isso já foi feito após o teste. No computador, `rm -f /tmp/freemytv-webos-armv7-probe`
remove o artefato; apagar `rust/probes/webos_armv7.rs` reverte a adição de código. Nenhum serviço, firmware ou
partição é alterado. Este probe só testa execução ARMv7/EABI e syscall no kernel. Por ser estático, não valida
glibc nem `/lib/ld-linux.so.3`; o probe dinâmico abaixo verifica essa fronteira.

## Probe dinâmico ARMv7 para webOS (issue #20)

`rust/probes/webos_armv7_dynamic.rs` chama `getpid()` da glibc e retorna `42` quando a chamada funciona. Os arquivos
`webos_armv7_libc_stub.c` e `.map` produzem uma biblioteca substituta **somente para a ligação local**; ela nunca é
enviada à TV. O carregador remoto resolve `libc.so.6` da própria TV. Compilar no computador Linux x86_64:

```bash
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
RUST_LLD="$(rustc --print sysroot)/lib/rustlib/$HOST_TRIPLE/bin/rust-lld"
clang --target=armv7-unknown-linux-gnueabi -mfloat-abi=soft -fPIC -c \
  rust/probes/webos_armv7_libc_stub.c -o /tmp/freemytv-webos-libc-link-stub.o
"$RUST_LLD" -flavor gnu -m armelf_linux_eabi -shared -soname libc.so.6 \
  --version-script rust/probes/webos_armv7_libc_stub.map \
  -o /tmp/freemytv-webos-libc-link-stub.so /tmp/freemytv-webos-libc-link-stub.o
rustc rust/probes/webos_armv7_dynamic.rs --target armv7-unknown-linux-gnueabi \
  -C panic=abort -C opt-level=s -C linker=rust-lld -C link-arg=-nostdlib \
  -C link-arg=-no-pie -C link-arg=-dynamic-linker=/lib/ld-linux.so.3 \
  -C link-arg=/tmp/freemytv-webos-libc-link-stub.so \
  -o /tmp/freemytv-webos-armv7-dynamic-probe
readelf -h -l -d -V /tmp/freemytv-webos-armv7-dynamic-probe
```

Verificação local em 2026-10-24: ELF32 ARM EABI5 soft-float, `PT_INTERP` `/lib/ld-linux.so.3`, `NEEDED` `libc.so.6`,
requisito `GLIBC_2.4`, sem `RPATH` nem `RUNPATH`. Após aprovação, definir `TV_IP` e `SSH_KEY` apenas no shell local,
transferir somente o executável e apagá-lo após o teste:

```bash
SSH_OPTS=(-p 9922 -i "$SSH_KEY" -o HostKeyAlgorithms=+ssh-rsa \
  -o PubkeyAcceptedAlgorithms=+ssh-rsa -o StrictHostKeyChecking=yes -o BatchMode=yes)
ssh "${SSH_OPTS[@]}" prisoner@"$TV_IP" \
  'umask 077; cat > /tmp/freemytv-webos-armv7-dynamic-probe && chmod 700 /tmp/freemytv-webos-armv7-dynamic-probe' \
  < /tmp/freemytv-webos-armv7-dynamic-probe
ssh "${SSH_OPTS[@]}" prisoner@"$TV_IP" \
  '/tmp/freemytv-webos-armv7-dynamic-probe; printf "probe_exit=%s\n" "$?"'
ssh "${SSH_OPTS[@]}" prisoner@"$TV_IP" \
  'rm -f /tmp/freemytv-webos-armv7-dynamic-probe && test ! -e /tmp/freemytv-webos-armv7-dynamic-probe'
```

Resultado na TV em 2026-10-24: `probe_exit=42`; remoção confirmada com `probe_removed=yes`. O carregador dinâmico
resolveu `libc.so.6` e `getpid@GLIBC_2.4` da TV. O teste não cobre outras funções da glibc ou o app completo.

Rollback: o último comando remove o único arquivo remoto criado; nenhuma biblioteca substituta é transferida. Os
artefatos locais ficam em `/tmp` e podem ser removidos. Este teste valida o carregador e a chamada `getpid` da glibc,
não todo o app Rust.

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

## Fase 2: a lógica do app web em WebAssembly (issue #14, revertida na #22)

**Status: revertido.** Regra do projeto: converter para Rust, a não ser que os testes, os comparativos ou a análise
mostrem regressão; nesse caso fica a versão original. A versão em WASM passou em todos os testes de paridade, mas
foi pior em desempenho, tamanho e abertura do app (números abaixo), então o app web voltou a ser JavaScript puro.
O código Rust (crate `freemytv-logic`, `web/logic.js`, `freemytv make-logic`) está no histórico, no commit `18bca6f`.
Abertura do app medida na TV LG de teste (webOS 10.3): +72 ms para decodificar o módulo em base64 e +28 ms para
compilá-lo, e cerca de 1 MB a mais de memória.

**Por que perdeu:** as funções são pequenas, e a ida e volta entre JS e WASM (copiar e converter textos, e
`JSON.stringify`/`parse` no clima) custa mais que a conta. O V8 já otimiza bem esse tipo de código, e o módulo
precisa trazer o que o navegador já tem (leitor de JSON, tabelas Unicode). WASM compensa em processamento pesado
com poucas idas e voltas (imagem, compressão, criptografia), o que o app web não tem.

### Como foi feito (para referência)

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

## Fase 2 no Android: protótipo JNI (issue #15, não convertida)

Antes de converter `HomeData`, `Recs` e `HomeMode`, um protótipo mediu duas funções reais do app em Java contra as
mesmas em Rust chamadas pelo JNI (`tests/bench/jni/run.sh`; JDK 17 no computador, porque não há TV Android aqui).
As respostas são iguais, conferidas pelo próprio protótipo.

| Função | Java | Rust (JNI) | Rust / Java |
|---|---:|---:|---:|
| `Recs.franchise` (sempre que a tela pede as sugestões) | 33 ns | 490 ns | 14,8× mais lento |
| `HomeData.parseWeather`, 48 h (a cada 30 min) | 60,8 µs | 13,0 µs | 4,7× mais rápido |

**Análise:**
- O único ganho está no `parseWeather`, que roda a cada 30 minutos: ~50 µs economizados, imperceptíveis. E é um
  ganho parcial: o Rust usou um fuso fixo, e para acertar o horário de verão como o Java (`TimeZone`) teria que levar
  o banco de fusos (mais tamanho) ou perguntar ao Java a cada hora (mais idas e voltas pelo JNI).
- Nas funções pequenas, a ponte JNI (converter e copiar textos) custa mais que a conta: até 15× mais lento.
- Custo fixo: +357 KB por processador no APK (ARM 32, ARM 64, x86, x86_64: até ~1,4 MB), o Android NDK no build e o
  `build.sh` reescrito, numa base que hoje compila só com o SDK. Pesa nas TVs de 1 GB.

**Decisão:** regressão, pela regra do projeto. O Android continua em Java. Como a fase 2 regrediu nas duas
plataformas, a fase 3 (interfaces, #16 e #17) não será feita: ela depende ainda mais do navegador e do Canvas do
Android, com mais idas e voltas e, nas TVs de 2017 a 2019, nem WebAssembly.

## Arte e símbolos: protótipo (issue #13, não convertida)

`tests/bench/art/run.sh` refaz em Rust (tiny-skia, image, ab_glyph) o `tools/make_art.py`: ícone do app, banner da TV
e imagens do site. Ele compara o resultado com as imagens do repositório, que o Pillow gera idênticas, como são
vistas sobre fundo branco e escuro.

Critério definido antes de medir: média abaixo de 1 nível (de 255) e 99,9% dos pixels a até 8 níveis.

| Imagem | Média (branco) | Máximo | Pixels a até 8 níveis |
|---|---:|---:|---:|
| `icon.png` (192) | 0,16 | 33 | 99,43% |
| `banner.png` (640×360) | 0,62 | 138 | 98,35% |
| `icon-512.png` | 0,09 | 40 | 99,72% |
| `apple-touch-icon.png` (180) | 0,17 | 32 | 99,33% |
| `favicon-64.png` | 0,38 | 28 | 98,88% |

Tempo: 11,0 s no Python e 0,9 s no Rust (12× mais rápido).

**Decisão:** regressão de fidelidade, pela regra do projeto. As bordas são rasterizadas de outro jeito (Pillow
desenha sem suavização e com regras próprias de pixel; o tiny-skia, com as dele) e o texto do banner fica ~1 px
deslocado. Gerar a arte com o Rust mudaria levemente os ícones já publicados. O ganho de tempo é num script que só
roda quando a arte muda, e traria três bibliotecas novas ao `freemytv`. `make_symbols.py` nem pôde ser comparado: a
fonte Material Symbols que ele usa não está no repositório. Os dois continuam em Python.
