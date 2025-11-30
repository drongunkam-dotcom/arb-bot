// dex.rs — интеграция с DEX'ами на Solana
// Назначение: получение котировок, расчёт маршрутов, подготовка транзакций

use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
pub struct DexQuote {
    pub dex_name: String,
    pub pair: String,
    pub input_amount: Decimal,
    pub output_amount: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
}

#[derive(Debug, Clone)]
pub struct DexClient {
    enabled_dexes: Vec<String>,
    trading_pairs: Vec<String>,
}

impl DexClient {
    pub fn new(config: &Config) -> Self {
        Self {
            enabled_dexes: config.dex.enabled_dexes.clone(),
            trading_pairs: config.dex.trading_pairs.clone(),
        }
    }
    
    /// Получение котировок со всех включённых DEX по всем парам
    pub async fn fetch_all_quotes(&self) -> Result<HashMap<String, Vec<DexQuote>>> {
        // ⚠️ Заглушка: реальная интеграция с DEX будет зависеть от конкретных протоколов (Raydium, Orca и т.д.)
        // Здесь мы оставляем структуру, чтобы можно было потом подключить реальные SDK/API.
        
        let mut result: HashMap<String, Vec<DexQuote>> = HashMap::new();
        
        for dex in &self.enabled_dexes {
            let mut quotes_for_dex = Vec::new();
            
            for pair in &self.trading_pairs {
                // В реальной реализации здесь будет RPC/HTTP запрос к DEX
                // Сейчас подставляем фиктивные значения для структуры
                let input_amount = Decimal::new(1000, 0); // 1000 единиц базового токена
                let price = Decimal::new(100, 2); // 1.00 условная цена
                let fee = Decimal::new(3, 2); // 0.03
                let one = Decimal::new(1, 0);
                let output_amount = input_amount * price * (one - fee);
                
                quotes_for_dex.push(DexQuote {
                    dex_name: dex.clone(),
                    pair: pair.clone(),
                    input_amount,
                    output_amount,
                    price,
                    fee,
                });
            }
            
            result.insert(dex.clone(), quotes_for_dex);
        }
        
        Ok(result)
    }
}