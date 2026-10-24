# App web (TVs LG webOS e Samsung Tizen)

A tela inicial das TVs LG e Samsung, em **JavaScript ES5 puro**. Não pode usar `fetch`, `Promise`, `let` nem
`includes`, porque o webOS 3/4 e o Tizen 3 rodam um Chromium de 2015 a 2017. O `tools/check.sh` garante isso.

## Ordem de carga (`web/index.html`)

```
config.js → coins.js → i18n.js → strings.js → tv.js → zones.js → data.js → home.js
(gerados pelo build)              (módulos do repositório)
```

Cada módulo é um objeto global criado por uma função autoexecutável (`var X = (function () {...})()`).

## Módulos

### `strings.js` → `I18N`, `tr`, `trf`, `plural`, `formatDate`
Traduções. A chave é o próprio texto em inglês.
- `tr('Live TV')`: texto traduzido, ou o inglês se faltar a tradução.
- `trf('{n} min ago', {n: 12})`: preenche placeholders.
- `plural('plural|one:{n} season|other:{n} seasons', n)`: escolhe a forma pelo `Intl.PluralRules`.
- `formatDate('format|h:mm a', data)`: formata com o padrão `SimpleDateFormat` do catálogo.
- `I18N.set('auto' | 'pt' | ...)`: troca o idioma.

### `tv.js` → `TV`
O lado "TV". Detecta a plataforma:
- **LG**: barramento Luna (`listLaunchPoints`, `launch`, entradas HDMI). No webOS 6+, quando o Luna nega a lista de
  apps, usa o WebSocket local `ws://127.0.0.1:3000` (pede permissão na TV uma vez).
- **Samsung**: `tizen.application`, `tizen.tvwindow`.
- **Navegador**: um substituto com apps de exemplo, para preview e testes.

### `zones.js` → `ZONES`
Gerado por `tools/make_zones.py`. Fuso → `"país,lat,lon[,cidade]"` ou `"=outro_fuso"` (apelido).

### `data.js` → `Place`, `Prefs`, `Data`
- **`Place`**: a localização. Usa o `config.properties` se ele tiver latitude; senão, o fuso da TV. Também deriva o
  país, a moeda (BR → BRL, zona do euro → EUR, demais → USD) e se usa Fahrenheit (EUA).
- **`Prefs`**: ajustes feitos na própria TV, salvos no `localStorage` (`fmtv.prefs`). Têm prioridade sobre o
  `config.properties`.
- **`Data`**: busca e guarda em cache:
  - clima, nascer e pôr do sol (Open-Meteo, a cada 30 min);
  - cotações de cripto (Binance, a cada 5 min);
  - sugestões (TMDB, 3 por hora, só de serviços instalados na TV).
  Funções de formatação puras: `compact`, `money`, `change`, `degrees`, `condition`, `symbol`, `isNight`,
  `staleness`.

### `home.js`
Desenha e controla a tela: abas, prateleira de apps, gaveta (entradas, clima, cotações), Home Settings, editor da
prateleira, navegação pelo controle remoto. Segue o layout do `HomeView` do Android.

### `home.css`
Estilos sem variáveis CSS, sem grid e sem `backdrop-filter`. Só `transform` e `opacity` são animados.

## Testes

`tests/web/` carrega estes arquivos num sandbox do Node (`vm`), com `window`, `localStorage` e `XMLHttpRequest`
falsos, e testa as funções públicas. Veja [testes.md](testes.md).
