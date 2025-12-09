/// Моки для RPC и DEX API
/// 
/// Этот модуль предоставляет моки для тестирования без реальных сетевых запросов

use anyhow::Result;
use rust_decimal::Decimal;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Мок RPC клиента для тестирования
pub struct MockRpcClient {
    /// Хранилище аккаунтов (pubkey -> account data)
    accounts: Arc<Mutex<HashMap<Pubkey, Vec<u8>>>>,
    /// Хранилище балансов (pubkey -> balance in lamports)
    balances: Arc<Mutex<HashMap<Pubkey, u64>>>,
    /// Флаг для симуляции ошибок
    should_fail: Arc<Mutex<bool>>,
    /// Счётчик вызовов для тестирования retry логики
    call_count: Arc<Mutex<u32>>,
}

impl MockRpcClient {
    /// Создание нового мок RPC клиента
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(Mutex::new(HashMap::new())),
            should_fail: Arc::new(Mutex::new(false)),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Установка баланса для аккаунта
    pub fn set_balance(&self, pubkey: &Pubkey, balance: u64) {
        let mut balances = self.balances.lock().unwrap();
        balances.insert(*pubkey, balance);
    }

    /// Установка данных аккаунта
    pub fn set_account_data(&self, pubkey: &Pubkey, data: Vec<u8>) {
        let mut accounts = self.accounts.lock().unwrap();
        accounts.insert(*pubkey, data);
    }

    /// Включение режима ошибок
    pub fn set_should_fail(&self, should_fail: bool) {
        let mut flag = self.should_fail.lock().unwrap();
        *flag = should_fail;
    }

    /// Получение счётчика вызовов
    pub fn get_call_count(&self) -> u32 {
        *self.call_count.lock().unwrap()
    }

    /// Сброс счётчика вызовов
    pub fn reset_call_count(&self) {
        let mut count = self.call_count.lock().unwrap();
        *count = 0;
    }

    /// Получение баланса (имитация RPC вызова)
    pub fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        let should_fail = *self.should_fail.lock().unwrap();
        if should_fail {
            anyhow::bail!("Симуляция ошибки RPC");
        }

        let balances = self.balances.lock().unwrap();
        Ok(*balances.get(pubkey).unwrap_or(&0))
    }

    /// Получение данных аккаунта (имитация RPC вызова)
    pub fn get_account_data(&self, pubkey: &Pubkey) -> Result<Vec<u8>> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;

        let should_fail = *self.should_fail.lock().unwrap();
        if should_fail {
            anyhow::bail!("Симуляция ошибки RPC");
        }

        let accounts = self.accounts.lock().unwrap();
        accounts.get(pubkey)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Аккаунт не найден"))
    }
}

/// Мок DEX для тестирования
pub struct MockDex {
    name: String,
    prices: Arc<Mutex<HashMap<(String, String), Decimal>>>,
    should_fail_get_price: Arc<Mutex<bool>>,
    should_fail_swap: Arc<Mutex<bool>>,
    swap_call_count: Arc<Mutex<u32>>,
}

impl MockDex {
    /// Создание нового мок DEX
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            prices: Arc::new(Mutex::new(HashMap::new())),
            should_fail_get_price: Arc::new(Mutex::new(false)),
            should_fail_swap: Arc::new(Mutex::new(false)),
            swap_call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Установка цены для торговой пары
    pub fn set_price(&self, base_token: &str, quote_token: &str, price: Decimal) {
        let mut prices = self.prices.lock().unwrap();
        prices.insert((base_token.to_string(), quote_token.to_string()), price);
    }

    /// Включение режима ошибок для get_price
    pub fn set_should_fail_get_price(&self, should_fail: bool) {
        let mut flag = self.should_fail_get_price.lock().unwrap();
        *flag = should_fail;
    }

    /// Включение режима ошибок для swap
    pub fn set_should_fail_swap(&self, should_fail: bool) {
        let mut flag = self.should_fail_swap.lock().unwrap();
        *flag = should_fail;
    }

    /// Получение счётчика вызовов swap
    pub fn get_swap_call_count(&self) -> u32 {
        *self.swap_call_count.lock().unwrap()
    }

    /// Получение цены (имитация DEX API)
    pub async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<Decimal> {
        let should_fail = *self.should_fail_get_price.lock().unwrap();
        if should_fail {
            anyhow::bail!("Симуляция ошибки получения цены");
        }

        let prices = self.prices.lock().unwrap();
        prices.get(&(base_token.to_string(), quote_token.to_string()))
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Цена не найдена для пары {}/{}", base_token, quote_token))
    }

    /// Выполнение свопа (имитация DEX API)
    pub async fn execute_swap(
        &self,
        _simulation_mode: bool,
        _from_token: &str,
        _to_token: &str,
        _amount: Decimal,
        _min_output: Decimal,
    ) -> Result<String> {
        let mut count = self.swap_call_count.lock().unwrap();
        *count += 1;

        let should_fail = *self.should_fail_swap.lock().unwrap();
        if should_fail {
            anyhow::bail!("Симуляция ошибки выполнения свопа");
        }

        Ok(format!("mock_signature_{}_{}", self.name, count))
    }
}

/// Вспомогательные функции для создания тестовых данных

/// Создание тестового ключа
pub fn create_test_keypair() -> Keypair {
    Keypair::new()
}

/// Создание тестового пула Raydium (заглушка данных)
pub fn create_mock_raydium_pool_data() -> Vec<u8> {
    // Упрощённая структура данных пула
    // В реальности это сложная структура с множеством полей
    vec![0u8; 512] // Заглушка
}

/// Создание тестового Whirlpool Orca (заглушка данных)
pub fn create_mock_orca_whirlpool_data() -> Vec<u8> {
    // Упрощённая структура данных Whirlpool
    vec![0u8; 512] // Заглушка
}

/// Создание тестового рынка Serum (заглушка данных)
pub fn create_mock_serum_market_data() -> Vec<u8> {
    // Упрощённая структура данных рынка
    vec![0u8; 512] // Заглушка
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_rpc_client() {
        let mock_rpc = MockRpcClient::new();
        let test_pubkey = Pubkey::new_unique();

        // Тест получения баланса
        mock_rpc.set_balance(&test_pubkey, 1_000_000_000);
        let balance = mock_rpc.get_balance(&test_pubkey).unwrap();
        assert_eq!(balance, 1_000_000_000);

        // Тест режима ошибок
        mock_rpc.set_should_fail(true);
        assert!(mock_rpc.get_balance(&test_pubkey).is_err());

        // Тест счётчика вызовов
        mock_rpc.set_should_fail(false);
        mock_rpc.reset_call_count();
        let _ = mock_rpc.get_balance(&test_pubkey);
        assert_eq!(mock_rpc.get_call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_dex() {
        let mock_dex = MockDex::new("test_dex");
        let price = Decimal::from(100);

        // Тест установки и получения цены
        mock_dex.set_price("SOL", "USDC", price);
        let retrieved_price = mock_dex.get_price("SOL", "USDC").await.unwrap();
        assert_eq!(retrieved_price, price);

        // Тест режима ошибок
        mock_dex.set_should_fail_get_price(true);
        assert!(mock_dex.get_price("SOL", "USDC").await.is_err());

        // Тест свопа
        mock_dex.set_should_fail_get_price(false);
        mock_dex.set_should_fail_swap(false);
        let signature = mock_dex.execute_swap(
            true,
            "SOL",
            "USDC",
            Decimal::from(1),
            Decimal::from(100),
        ).await.unwrap();
        assert!(signature.contains("mock_signature"));
        assert_eq!(mock_dex.get_swap_call_count(), 1);
    }
}

