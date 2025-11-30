// wallet.rs — управление кошельком Solana
// Назначение: загрузка ключей, подписание транзакций, проверка баланса

use anyhow::{Result, Context};
use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use solana_client::rpc_client::RpcClient;
use std::fs;
use std::sync::Arc;

use crate::config::Config;

pub struct Wallet {
    keypair: Keypair,
    rpc_client: Arc<RpcClient>,
}

impl Wallet {
    /// Создание нового экземпляра кошелька
    pub fn new(config: &Config) -> Result<Self> {
        // Загрузка keypair из файла
        let keypair = Self::load_keypair(&config.wallet.keypair_path)?;
        
        // Создание RPC клиента
        let rpc_client = Arc::new(RpcClient::new(config.network.rpc_url.clone()));
        
        Ok(Self {
            keypair,
            rpc_client,
        })
    }
    
    /// Загрузка keypair из JSON файла
    fn load_keypair(path: &str) -> Result<Keypair> {
        // Проверка прав доступа к файлу (должен быть 400 или 600)
        let metadata = fs::metadata(path)
            .context(format!("Не удалось прочитать файл ключа: {}", path))?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            if mode & 0o077 != 0 {
                log::warn!(
                    "⚠️  Файл ключа {} имеет небезопасные права доступа! Установите: chmod 400 {}",
                    path,
                    path
                );
            }
        }
        
        // Чтение и парсинг keypair
        let keypair_bytes = fs::read_to_string(path)
            .context("Не удалось прочитать содержимое файла ключа")?;
        
        let keypair_vec: Vec<u8> = serde_json::from_str(&keypair_bytes)
            .context("Некорректный формат файла ключа (ожидается JSON массив)")?;
        
        Keypair::from_bytes(&keypair_vec).context("Не удалось создать keypair из байтов")
    }
    
    /// Получение публичного ключа
    pub fn public_key(&self) -> Pubkey {
        self.keypair.pubkey()
    }
    
    /// Получение баланса в SOL
    pub async fn get_balance(&self) -> Result<f64> {
        let lamports = self
            .rpc_client
            .get_balance(&self.keypair.pubkey())
            .context("Не удалось получить баланс")?;
        
        // Конвертация lamports в SOL (1 SOL = 1_000_000_000 lamports)
        Ok(lamports as f64 / 1_000_000_000.0)
    }
    
    /// Получение ссылки на keypair для подписания транзакций
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }
    
    /// Получение ссылки на RPC клиент
    pub fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }
}