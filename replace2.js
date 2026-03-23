const fs = require('fs');
let s = fs.readFileSync('web/src/components/trading/TradingChart.svelte', 'utf8');

s = s.replace(`import { fetchCandles } from '../../lib/api/client';`, 
  `import { fetchCandles } from '../../lib/api/client';\n    let { market = "BTC_USDT" } = $props<{ market?: string }>();`);

s = s.replace(`fetchCandles("BTC_USDT"`, `fetchCandles(market`);

fs.writeFileSync('web/src/components/trading/TradingChart.svelte', s);
console.log('done');
