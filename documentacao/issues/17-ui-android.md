---
title: "Avaliar conversão da interface Android para Rust"
labels: [fase-3-interface, conversao-rust]
status: descartado
---

**Origem:** HomeView, DreamView, SettingsActivity, Scene/Shaders
**Destino:** a definir

Meta de longo prazo. A tela, o protetor de tela e os serviços dependem de APIs Java; uma casca Java vai continuar existindo.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
