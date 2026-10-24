---
title: "Converter gerador de fusos horários"
labels: [fase-1-ferramentas, conversao-rust]
status: feito
---

**Origem:** tools/make_zones.py
**Destino:** rust/crates/freemytv-core/src/zones.rs + `freemytv make-zones`

Gerar `web/zones.js` a partir do banco de fusos (`zone.tab`, `tzdata.zi`).

## Critérios de aceite
- [x] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [x] Testes unitários no crate
- [x] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [x] Benchmark em `tests/bench/bench.py`, quando aplicável
