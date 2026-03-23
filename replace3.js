const fs = require('fs');
let s = fs.readFileSync('web/src/components/trade/RecentTradesPanel.svelte', 'utf8');

s = s.replace(`base_asset: "BTC"`, `base_asset: $selectedMarket.split("_")[0]`);
s = s.replace(`import { orderBook } from "../../stores/orderBookStore";`, `import { orderBook } from "../../stores/orderBookStore";\n  import { selectedMarket } from "../../stores/marketStore";`);

fs.writeFileSync('web/src/components/trade/RecentTradesPanel.svelte', s);
