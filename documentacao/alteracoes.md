# Registro de alterações do fork

Tudo o que este fork muda em relação ao upstream (`simbasjungles/free-my-tv`). O código original não foi alterado;
as mudanças ficam em `documentacao/`, `tests/` e `rust/`.

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
- Itens 09 a 17 (veja `rust.md`). O próximo é o instalador LG em Rust, com a correção da verificação da host key.
