---
title: "Converter montagem dos arquivos do app web"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/webapp.py: files, coins, preview
**Destino:** rust/crates/freemytv-core/src/webapp.rs

Juntar `web/`, ícones, `config.js`, `coins.js`, `i18n.js`, símbolos e fonte.

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
