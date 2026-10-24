---
title: "Avaliar conversão da interface web (home.js) para Rust"
labels: [fase-3-interface, conversao-rust]
status: descartado
---

**Origem:** web/home.js, web/home.css
**Destino:** a definir (Leptos/Yew ou manter JS)

Meta de longo prazo. Medir antes se há ganho: a interface depende do motor do navegador da TV.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
