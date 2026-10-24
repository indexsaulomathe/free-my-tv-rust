# Registro de alterações do fork

Tudo o que este fork muda em relação ao upstream (`simbasjungles/free-my-tv`). O código original não foi alterado;
as mudanças ficam no README, em `documentacao/`, `tests/` e `rust/`.

## 2026-10-24: nome `rust-free-my-tv` (issue #26)
- Repositório no GitHub renomeado de `free-my-tv-rust` para `rust-free-my-tv`, preservando o vínculo de fork.
- README e valor padrão do script de criação de issues atualizados para o novo nome. Identificadores internos do app e
  nome do CLI não mudaram, para preservar compatibilidade de instalação e atualização.
- Reversão: renomear o repositório de volta no GitHub, restaurar URL do remoto local e os nomes alterados neste commit.

## 2026-10-24: GitHub Actions ativado no fork (issue #23)
- A configuração de Actions foi reativada no fork pela API do GitHub. O push seguinte iniciou o workflow `Check`.
- Execução `40083518280`: jobs Android e web concluídos com sucesso. Nenhum arquivo de workflow foi alterado.
- Reversão: desativar Actions nas configurações do repositório, se desejado.

## 2026-10-24: About do repositório (issue #25)
- Descrição no GitHub atualizada para identificar o fork e a conversão gradual das ferramentas para Rust.
- Link de site do upstream removido do About deste fork; tópicos adicionados: `android-tv`, `google-tv`, `rust`,
  `tizen` e `webos`.
- Nenhum arquivo do app, vínculo de fork ou release foi alterado. Reversão: restaurar descrição, site e tópicos
  anteriores nas configurações do repositório. A descrição anterior era “A fast, ad-free home screen for Google TV,
  LG webOS and Samsung Tizen. One button to undo.”; o site anterior era `https://freemytv.io`; não havia tópicos.

## 2026-10-24: README do fork Rust (issue #24)
- README reescrito para identificar o repositório como fork independente baseado em `simbasjungles/free-my-tv`.
- Instalação e desenvolvimento agora apontam para este fork; comandos usam placeholders para endereços e perfis.
- Estado da conversão, limites dos testes em TV e simulador, crédito ao upstream e reversão documentados.
- Vínculo de fork no GitHub mantido; nenhum código de app ou firmware foi alterado.

## 2026-10-24: probe dinâmico ARMv7 para webOS (issue #20)
- Probe Rust liga contra biblioteca substituta local com `getpid@GLIBC_2.4`, sem copiar bibliotecas da TV.
- ELF local exige `libc.so.6` e `/lib/ld-linux.so.3`; teste na TV retornou `probe_exit=42`.
- Único executável remoto removido de `/tmp`, com ausência confirmada.
- Build, verificação e rollback documentados em `rust.md` sem registrar IP ou caminho de chave.

## 2026-10-24: probe Rust ARMv7 para webOS (issue #19)
- Executável mínimo `no_std` para testar ELF32 ARM EABI soft-float como usuário `prisoner`.
- Build local, procedimento de teste remoto, limites e rollback documentados em `rust.md`.
- Teste real em TV LG webOS: saída `probe_exit=42`; cópia por `cat` sobre SSH porque `scp` não existe na TV;
  executável removido de `/tmp` após o teste.

## 2026-10-24: arte e símbolos ficam em Python (issue #13)
- Protótipo `tests/bench/art/`: o `make_art.py` em Rust é 12× mais rápido (0,9 s contra 11 s), mas não reproduz as
  imagens publicadas (98,4% a 99,7% dos pixels a até 8 níveis; o critério era 99,9%), e o texto do banner fica ~1 px
  deslocado. É regressão de fidelidade: fica em Python, assim como o `make_symbols.py`.

## 2026-10-24: Android e interfaces ficam como estão (issues #15, #16, #17)
- Protótipo JNI (`tests/bench/jni/`): `Recs.franchise` ficou 14,8× mais lento em Rust e `HomeData.parseWeather`
  4,7× mais rápido, num caminho que roda a cada 30 minutos. Somando +357 KB por processador no APK, o NDK no build e o
  fuso horário sem horário de verão, deu regressão: o Android continua em Java (#15).
- Pela mesma regra, as interfaces (#16 `home.js`, #17 `HomeView`) não serão convertidas.

## 2026-10-24: lógica do app web volta para JavaScript (issue #22)
- Revertida a #14: saíram `web/logic.js`, `web/logic_wasm.js`, as chamadas ao `Logic` em `data.js` e `strings.js`,
  o crate `freemytv-logic`, o `freemytv make-logic` e os testes e o benchmark do WASM. `web/` está de novo igual ao
  original. O código fica no histórico (`18bca6f`).
- Motivo: regressão medida. De 3 a 11 vezes mais lento por chamada, +90 KB no `.ipk`, +~100 ms na abertura do app
  e +1 MB de memória na TV. Regra do projeto: converter para Rust, exceto quando houver regressão nos testes, nos
  comparativos ou na análise.

## 2026-10-24: lógica do app web em Rust/WebAssembly (issue #14)
- Novo crate `freemytv-logic` com as funções puras de `web/data.js` e `web/strings.js` (clima, palavras e ícones do
  tempo, regras das sugestões, plurais, escolha do idioma), compilado para WebAssembly.
- `freemytv make-logic` gera `web/logic_wasm.js`; `web/logic.js` o carrega e expõe `Logic`. `data.js` e
  `strings.js` perguntam ao `Logic` primeiro e usam o JavaScript como reserva (TVs sem WebAssembly, enquanto carrega,
  entradas estranhas). `strings.js` ganhou `I18N.text(chave)` para traduzir uma chave guardada numa variável.
- `index.html` carrega `logic_wasm.js` e `logic.js` antes de `strings.js`.
- Testes: a suíte web roda em JS e em WASM (`tests/run.sh`), mais 6 testes de paridade (`tests/web/logic.test.js`)
  e 6 testes Rust. Benchmark novo: `tests/bench/logic_bench.js`.
- Testado numa TV LG de teste (webOS 10.3): o WASM carrega e responde certo.
- Medido: de 3 a 11 vezes mais lento que o JS por chamada (micro/nanossegundos, imperceptível); o `.ipk` cresce 90 KB.
- O perfil `release` passou a usar `panic = "abort"` e o crate WASM `opt-level = "s"`.

## 2026-10-24: envio de fotos em Rust (issue #12)
- `freemytv photos [IP] FOTOS_OU_PASTAS...` substitui `tools/photos.py`: mesma ordem (pastas como o `os.walk`, sem
  ocultos), mesmo nome na TV (`000-nome_sem_espacos`), `adb push` na pasta do app ou `content write` na PhotoInbox
  (Android 11, Fire OS 8), com o nome codificado como o `urllib.parse.quote` e o tamanho na URI.
- Paridade testada com um `adb` falso em 5 situações (envio direto, pela PhotoInbox, app antigo, app não instalado,
  conexão recusada): mesma saída, mesmo código de saída e os mesmos comandos do adb, na mesma ordem.
- 3 testes Rust novos. Não testado numa TV Android de verdade (a TV daqui é LG).

## 2026-10-24: validação das traduções em Rust (issue #11)
- `freemytv translate-check` substitui `tools/translate.py --check`: extrai as chaves do Java (`Ui.tr`, `trf`,
  `plural`, `datePattern`) e do `web/`, e valida todos os catálogos (chaves faltando ou obsoletas, placeholders,
  plurais CLDR, padrões do SimpleDateFormat, `en.json` desatualizado). `--fill`, `--prune` e `--new` continuam no Python.
- Saída idêntica à do Python, inclusive os avisos e as chaves escritas como o `repr` do Python: 2 testes de paridade
  novos (`test_rust_parity.py`), um com os catálogos do repositório e outro com 9 tipos de erro de propósito.
- `i18n.rs` ganhou `web_calls_at` (linha e função de cada chamada) e `literal_value` (Java aceita só aspas duplas).
- 4 testes Rust novos. Benchmark: 9,2× mais rápido que o Python (`tests/bench/bench.py`).

## 2026-10-24: instalador Samsung em Rust (issue #10)
- `freemytv tizen-install IP [--uninstall]` substitui `tizen/install.py`: o mesmo protocolo SDB (cabeçalho de 24
  bytes, OPEN/OKAY/WRTE/CLSE, `sync:` SEND/DATA/DONE), `0 vd_appinstall`, `0 was_execute` e as mesmas mensagens.
- O sucesso só vale com `install completed` (como no Python: `cmd_ret:0` também aparece depois de falhas).
- 4 testes novos: cabeçalho igual ao `struct.pack` do Python, ids do manifesto, capacidades, respostas do instalador
  e uma conversa inteira com uma TV simulada (conexão, capacidades, envio de 5000 bytes em pacotes e instalação).
- Não testado numa Samsung de verdade (não há uma aqui).

## 2026-10-24: instalador LG em Rust (issue #9)
- `freemytv webos-install IP [--uninstall]` substitui `webos/install.py`: chave pelo Key Server, cópia por SSH e
  instalação pelo `com.webos.appInstallService`, com as mesmas mensagens.
- Segurança (achado #4): `StrictHostKeyChecking=accept-new` com `~/.ssh/freemytv_webos_known_hosts` em vez de `no`;
  a passphrase é pedida pelo `ssh-keygen` (o Python a passava em `-P`, visível no `ps`); a chave é criada já com 0600.
- Testado numa TV LG de teste: instalou e abriu duas vezes, e uma host key trocada foi recusada.
- 6 testes novos (`webos_install.rs`): IP, caminho da chave igual ao do Python, opções do ssh, linhas do Luna,
  respostas da TV e do Key Server.

## 2026-10-24: clippy e cargo fmt (issue #18)
- O Rust da fase 1 passa em `cargo clippy -- -D warnings` e `cargo fmt --check`, sem mudar comportamento.

## 2026-10-24: branch `claude/testes-docs-rust`

### Documentação (`documentacao/`)
- Mapa dos componentes e um documento por camada: ferramentas, app web, app Android e site.
- Análise de segurança com 7 riscos classificados e recomendações.
- Guia de testes e plano de conversão para Rust.
- Uma issue por item a converter (`documentacao/issues/`, 17 itens) e o script `criar-issues.sh`, que as abre no
  GitHub.

### Testes (`tests/`)
- 60 testes Python (`tests/python/`), incluindo a paridade Rust × Python.
- 23 testes do app web em sandbox Node (`tests/web/`).
- `tests/run.sh` roda tudo; `tests/bench/bench.py` mede Python × Rust.

### Conversão para Rust (`rust/`), fase 1, itens 01 a 08
- Crate `freemytv-core`: config, JSON igual ao do Python, catálogos i18n, arquivos do app web, pacotes `.ipk` e
  `.wgt`, repositório do Homebrew e fusos horários.
- CLI `freemytv` com os comandos `webos-build`, `tizen-build`, `homebrew-repo` e `make-zones`.
- Saída idêntica à do Python. Ganho de 2,5× a 9× no tempo e de 3× a 4× na memória (veja `rust.md`).

### Pendente
- Nada da conversão. 12 itens em Rust; 13 a 17 avaliados e mantidos como estão (regressão), o 14 revertido (#22).
