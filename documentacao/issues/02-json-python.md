---
title: "Escritor JSON idêntico ao json.dumps do Python"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** json.dumps (biblioteca padrão do Python)
**Destino:** rust/crates/freemytv-core/src/json.rs

Os arquivos gerados (`config.js`, `i18n.js`, `apps.json`, `zones.js`) precisam sair byte a byte iguais. O serde_json usa outro espaçamento e outros escapes, então o crate tem um escritor próprio (`ensure_ascii`, `separators`, `indent`).

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
