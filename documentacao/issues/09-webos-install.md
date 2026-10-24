---
title: "Converter instalador LG e corrigir verificação da host key"
labels: [fase-1-ferramentas,seguranca, conversao-rust]
status: feito
---

**Origem:** webos/install.py
**Destino:** `freemytv webos-install IP [--uninstall]`

Baixar a chave pelo Key Server, destravar com a passphrase, copiar por SSH e instalar via Luna.

**Correção de segurança (achado #4 de `documentacao/seguranca.md`):** trocar `StrictHostKeyChecking=no` por `accept-new` com um `known_hosts` próprio, para a TV ser verificada a partir da segunda conexão. Manter a chave com permissão 0600.

## Critérios de aceite
- [ ] Mesma saída do Python (teste de paridade em `tests/python/test_rust_parity.py`, quando houver arquivo gerado)
- [ ] Testes unitários no crate
- [ ] Documentado em `documentacao/rust.md` e `documentacao/alteracoes.md`
- [ ] Benchmark em `tests/bench/bench.py`, quando aplicável
