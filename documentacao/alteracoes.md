# Registro de alterações do fork

Tudo o que este fork muda em relação ao upstream (`simbasjungles/free-my-tv`). O código original não foi alterado;
as mudanças ficam em `documentacao/`, `tests/` e `rust/`.

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
- Testado na TV LG 192.168.4.9: instalou e abriu duas vezes, e uma host key trocada foi recusada.
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
- Itens 12 a 17 (veja `rust.md`).
