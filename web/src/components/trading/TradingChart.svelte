<script lang="ts">
    import { createChart, ColorType, type IChartApi, type CandlestickSeriesPartialOptions, type HistogramSeriesPartialOptions, type UTCTimestamp, type ISeriesApi, type TickMarkType, type Time } from 'lightweight-charts';
    import { onMount, onDestroy } from 'svelte';
    import { fetchCandles } from '../../lib/api/client';
    import { orderBook } from '../../stores/orderBookStore';
    let { market = "BTC_USDT" } = $props<{ market?: string }>();

    // Props structure for OHLCV data
    interface CandleData {
        time: UTCTimestamp;
        open: number;
        high: number;
        low: number;
        close: number;
        value: number; // Volume
    }

    let chartContainer: HTMLDivElement;
    let chart: IChartApi;
    
    let candlestickSeries: ISeriesApi<"Candlestick">;
    let volumeSeries: ISeriesApi<"Histogram">;

    let activeCandle: CandleData | null = null;
    let unsubscribeWs: () => void;

    const localTimeFormatter = (unixSeconds: number): string => {
        return new Date(unixSeconds * 1000).toLocaleTimeString([], {
            hour: '2-digit',
            minute: '2-digit',
            hour12: false,
        });
    };

    const toUnixSeconds = (time: Time): number => {
        if (typeof time === 'number') {
            return time;
        }
        if (typeof time === 'string') {
            return Math.floor(new Date(`${time}T00:00:00`).getTime() / 1000);
        }
        return Math.floor(new Date(time.year, time.month - 1, time.day).getTime() / 1000);
    };

    onMount(async () => {
        if (!chartContainer) return;

        // 1. Initialize the Chart
        chart = createChart(chartContainer, {
            layout: {
                background: { type: ColorType.Solid, color: '#0A0E17' }, // Cyberpunk dark navy
                textColor: '#B2B5BE', // Light gray text
            },
            grid: {
                vertLines: { color: '#1E222D' }, // Subtle grid
                horzLines: { color: '#1E222D' },
            },
            timeScale: {
                timeVisible: true,
                secondsVisible: false,
                borderColor: '#1E222D',
                tickMarkFormatter: (time: Time, _tickMarkType: TickMarkType, _locale: string): string => {
                    return localTimeFormatter(toUnixSeconds(time));
                },
            },
            localization: {
                locale: navigator.language,
                timeFormatter: (time: number): string => localTimeFormatter(time),
            },
            rightPriceScale: {
                borderColor: '#1E222D',
            },
            width: chartContainer.clientWidth,
            height: chartContainer.clientHeight,
        });

        // 2. Configure Candlestick Series
        candlestickSeries = chart.addCandlestickSeries({
            upColor: '#00FF7F', // Neon Green
            downColor: '#FF3B69', // Neon Pink/Red
            borderVisible: false,
            wickUpColor: '#00FF7F',
            wickDownColor: '#FF3B69',
        } as CandlestickSeriesPartialOptions);

        // 3. Configure Volume Histogram Overlay
        volumeSeries = chart.addHistogramSeries({
            color: '#26a69a',
            priceFormat: {
                type: 'volume',
            },
            priceScaleId: '', // Set as an overlay
        } as HistogramSeriesPartialOptions);

        volumeSeries.priceScale().applyOptions({
            scaleMargins: {
                top: 0.75, // Leave 75% for candles, 25% for volume at the bottom
                bottom: 0,
            },
        });

        // 4. Handle Responsiveness using ResizeObserver
        const resizeObserver = new ResizeObserver((entries) => {
            const entry = entries[0];
            if (!entry || entry.target !== chartContainer) return;
            const newRect = entry.contentRect;
            chart.applyOptions({ width: newRect.width, height: newRect.height });
        });

        resizeObserver.observe(chartContainer);

        // 5. Fetch historical candles from DB
        try {
            const apiCandles = await fetchCandles(market, "1m", 100);
            
            // Map and sort chronologically (oldest first required by lightweight-charts)
            const historicalData: CandleData[] = apiCandles
                .map(c => ({
                    time: (c.time / 1000) as UTCTimestamp,
                    open: parseFloat(c.open),
                    high: parseFloat(c.high),
                    low: parseFloat(c.low),
                    close: parseFloat(c.close),
                    value: parseFloat(c.volume)
                }))
                .sort((a, b) => (a.time as number) - (b.time as number));

            if (historicalData.length > 0) {
                const lastIdx = historicalData.length - 1;
                const lastHData = historicalData[lastIdx];
                if (lastHData) {
                    activeCandle = { ...lastHData };
                }
                
                candlestickSeries.setData(historicalData);
                volumeSeries.setData(
                    historicalData.map(d => ({
                        time: d.time,
                        value: d.value,
                        color: d.close >= d.open ? '#00FF7F80' : '#FF3B6980', 
                    }))
                );
            }
        } catch (err) {
            console.error("Failed to fetch historical candles", err);
        }

        // 6. Subscribe to real-time orderBook trades to update the chart dynamically
        let initialLoad = true;
        let lastProcessedTradeTs: number | null = null;

        unsubscribeWs = orderBook.subscribe(state => {
            const isInitial = initialLoad;
            initialLoad = false;

            const trades = state.trades;
            if (trades.length === 0) return;

            const latestTrade = trades[0];
            if (!latestTrade) return;
            const tradeMs = latestTrade.ts || Date.now();

            // Svelte runs subscribe synchronously on attach. Ignore the immediate existing state since 
            // the historical fetchCandles handles past data.
            if (isInitial) {
                lastProcessedTradeTs = tradeMs;
                return; 
            }

            // Only process if it's a genuinely new trade (prevent duplicates from orderbook depth updates)
            if (lastProcessedTradeTs === tradeMs) return;
            lastProcessedTradeTs = tradeMs;

            const tradePrice = latestTrade.price;
            const tradeVolume = latestTrade.amount;
            
            // 1-minute aggregation interval (in seconds for UTCTimestamp)
            const candleTimeSeconds = Math.floor(tradeMs / 60000) * 60 as UTCTimestamp;

            if (activeCandle && activeCandle.time === candleTimeSeconds) {
                // Update current candle
                activeCandle.close = tradePrice;
                activeCandle.high = Math.max(activeCandle.high, tradePrice);
                activeCandle.low = Math.min(activeCandle.low, tradePrice);
                activeCandle.value += tradeVolume;
            } else {
                // Start a new candle based on the last one
                activeCandle = {
                    time: candleTimeSeconds,
                    open: activeCandle ? activeCandle.close : tradePrice, // Open at previous close
                    high: tradePrice,
                    low: tradePrice,
                    close: tradePrice,
                    value: tradeVolume
                };
            }

            // Apply updates
            candlestickSeries.update(activeCandle);
            volumeSeries.update({
                time: activeCandle.time,
                value: activeCandle.value,
                color: activeCandle.close >= activeCandle.open ? '#00FF7F80' : '#FF3B6980'
            });
        });

        onDestroy(() => {
            if (unsubscribeWs) unsubscribeWs();
            resizeObserver.disconnect();
            chart.remove();
        });
    });
</script>

<div bind:this={chartContainer} class="w-full h-full min-h-100"></div>
