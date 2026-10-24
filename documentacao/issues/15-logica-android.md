---
title: "Converter lógica pura do Android para Rust (JNI)"
labels: [fase-2-logica, conversao-rust]
status: descartado
---

**Origem:** HomeData, Recs (parsing e regras), regras do HomeMode (mayTurnOff, listas)
**Destino:** crate `freemytv-logic` + biblioteca nativa via NDK/JNI

Mesmo crate da issue de lógica web, compilado para Android. Precisa do Android NDK.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
