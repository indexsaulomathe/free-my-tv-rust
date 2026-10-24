# Registro de alterações do fork

Tudo o que este fork muda em relação ao upstream (`simbasjungles/free-my-tv`). O código original não foi alterado;
as mudanças ficam em `documentacao/`, `tests/` e `rust/`.

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
- Itens 10 a 17 (veja `rust.md`).
