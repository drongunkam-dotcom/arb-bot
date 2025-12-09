use anyhow::{Context, Result};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::fs;
use std::path::PathBuf;
use crate::config::Config;

/// Управление кошельком Solana
pub struct Wallet {
    keypair: Keypair,
    pubkey: Pubkey,
}

impl Wallet {
    /// Создание нового экземпляра кошелька из конфигурации
    pub fn new(config: &Config) -> Result<Self> {
        let key_path = &config.wallet.keypair_path;

        // Проверка существования файла
        if !key_path.exists() {
            anyhow::bail!("Файл ключа не найден: {:?}", key_path);
        }

        // Проверка прав доступа (на Unix-системах)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(key_path)
                .with_context(|| format!("Не удалось получить метаданные файла: {:?}", key_path))?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // Проверка, что файл доступен только владельцу (0o400 или 0o600)
            if mode & 0o077 != 0 {
                log::warn!("⚠️  Файл ключа имеет слишком открытые права доступа: {:o}", mode);
            }
        }

        // Чтение ключа
        let key_bytes = fs::read(key_path)
            .with_context(|| format!("Не удалось прочитать файл ключа: {:?}", key_path))?;

        // Парсинг ключа (может быть JSON или raw bytes)
        let keypair = if key_bytes[0] == b'{' {
            // JSON формат
            let json: serde_json::Value = serde_json::from_slice(&key_bytes)
                .context("Ошибка парсинга JSON ключа")?;
            let key_array: Vec<u8> = serde_json::from_value(
                json.get("secretKey")
                    .ok_or_else(|| anyhow::anyhow!("Поле secretKey не найдено"))?
                    .clone()
            )
            .context("Ошибка парсинга secretKey")?;
            Keypair::from_bytes(&key_array)
                .context("Ошибка создания Keypair из байтов")?
        } else {
            // Raw bytes (64 байта)
            Keypair::from_bytes(&key_bytes)
                .context("Ошибка создания Keypair из байтов")?
        };

        let pubkey = keypair.pubkey();

        log::info!("Кошелёк загружен: {}", pubkey);

        Ok(Self { keypair, pubkey })
    }

    /// Получение публичного ключа
    pub fn pubkey(&self) -> &Pubkey {
        &self.pubkey
    }

    /// Получение ключевой пары (для подписания транзакций)
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Получение баланса кошелька
    pub async fn get_balance(&self, rpc_url: &str) -> Result<u64> {
        use solana_client::rpc_client::RpcClient;
        use solana_sdk::commitment_config::CommitmentConfig;

        let client = RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        );

        let balance = client.get_balance(&self.pubkey)
            .context("Не удалось получить баланс")?;

        Ok(balance)
    }
}

