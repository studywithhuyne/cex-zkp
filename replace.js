const fs = require('fs');

let s = fs.readFileSync('web/src/components/trade/TradeFormPanel.svelte', 'utf8');

s = s.replace(`import { orderBook } from '../../stores/orderBookStore';`, 
  `import { orderBook } from '../../stores/orderBookStore';\n  import { selectedMarket } from '../../stores/marketStore';`);

s = s.replace(/let btcAvailable  = \$state\("0\.000"\);/, 
  'let baseAvailable  = $state("0.000");\n  let baseAsset = $derived($selectedMarket.split("_")[0]);');

s = s.replace(/b\.asset === "BTC"/g, 'b.asset === baseAsset');
s = s.replace(/const btc  = balances\.find\(b => b\.asset === baseAsset\);/, 'const base  = balances.find(b => b.asset === baseAsset);');
s = s.replace(/if \(btc\)  btcAvailable  = parseFloat\(btc\.available\)\.toFixed\(3\);/, 'if (base)  baseAvailable  = parseFloat(base.available).toFixed(3);');

s = s.replace(/fetchAveragePrice\("BTC_USDT"\)/, 'fetchAveragePrice($selectedMarket)');

s = s.replace(/pct of BTC available/, 'pct of base available');
s = s.replace(/const btc = parseFloat\(btcAvailable\);/, 'const baseAmount = parseFloat(baseAvailable);');
s = s.replace(/if \(btc > 0\) \{/g, 'if (baseAmount > 0) {');
s = s.replace(/amount = \(\(btc \* pct \/ 100\)\)\.toFixed\(6\);/, 'amount = ((baseAmount * pct / 100)).toFixed(6);');

s = s.replace(/base_asset:\s+"BTC"/g, 'base_asset: baseAsset');
s = s.replace(/Filled \$\{matched\.toFixed\(6\)\} BTC/g, 'Filled ${matched.toFixed(6)} ${baseAsset}');

s = s.replace(/Spot · BTC\/USDT/g, 'Spot · {baseAsset}/USDT');
s = s.replace(/Spot Â· BTC\/USDT/g, 'Spot · {baseAsset}/USDT'); // handle weird encoding just in case

s = s.replace(/\{btcAvailable\} BTC/g, '{baseAvailable} {baseAsset}');
s = s.replace(/>BTC<\/span>/g, '>{baseAsset}</span>');
s = s.replace(/Buy BTC/g, 'Buy {baseAsset}');
s = s.replace(/Sell BTC/g, 'Sell {baseAsset}');

fs.writeFileSync('web/src/components/trade/TradeFormPanel.svelte', s);
console.log("Done");
