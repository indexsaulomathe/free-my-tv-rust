---
title: "Converter extração de chaves e catálogos do app web"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/translate.py: web_calls, java_args, java_string, web_keys; tools/webapp.py: catalogues
**Destino:** rust/crates/freemytv-core/src/i18n.rs

Achar as chamadas `tr`, `trf`, `plural` e `formatDate` em `web/*.js` e montar o `i18n.js` só com as chaves usadas.

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
