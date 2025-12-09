use anyhow::{Context, Result};
use rust_decimal::Decimal;
use std::str::FromStr;
use solana_sdk::{
    pubkey::Pubkey,
    instruction::{Instruction, AccountMeta},
    transaction::Transaction,
    system_program,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use crate::config::Config;
use crate::wallet::Wallet;

/// Унифицированный интерфейс для работы с DEX
#[async_trait::async_trait]
pub trait DexInterface: Send + Sync {
    /// Получение названия DEX
    fn name(&self) -> &str;

    /// Получение цены для торговой пары
    /// Возвращает цену в формате: сколько quote_token за 1 base_token
    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<Decimal>;

    /// Выполнение свопа
    /// simulation_mode: если true, только симулирует транзакцию, не отправляет
    /// wallet: кошелёк для подписания транзакций
    async fn execute_swap(
        &self,
        simulation_mode: bool,
        from_token: &str,
        to_token: &str,
        amount: Decimal,
        min_output: Decimal,
        wallet: &Wallet,
    ) -> Result<String>; // Возвращает signature транзакции
}

/// Менеджер DEX
pub struct DexManager {
    dexes: Vec<Box<dyn DexInterface>>,
    config: Config,
}

impl DexManager {
    /// Создание нового менеджера DEX
    pub fn new(config: &Config) -> Result<Self> {
        let mut dexes: Vec<Box<dyn DexInterface>> = Vec::new();

        // Регистрация DEX согласно конфигурации
        for dex_name in &config.dex.enabled_dexes {
            match dex_name.as_str() {
                "raydium" => {
                    dexes.push(Box::new(RaydiumDex::new(config)?));
                }
                "orca" => {
                    dexes.push(Box::new(OrcaDex::new(config)?));
                }
                "serum" => {
                    dexes.push(Box::new(SerumDex::new(config)?));
                }
                _ => {
                    log::warn!("Неизвестный DEX: {}, пропускаем", dex_name);
                }
            }
        }

        log::info!("Зарегистрировано {} DEX", dexes.len());

        Ok(Self {
            dexes,
            config: config.clone(),
        })
    }

    /// Получение всех зарегистрированных DEX
    pub fn get_dexes(&self) -> &[Box<dyn DexInterface>] {
        &self.dexes
    }

    /// Получение DEX по имени
    pub fn get_dex(&self, name: &str) -> Option<&dyn DexInterface> {
        self.dexes.iter()
            .find(|dex| dex.name() == name)
            .map(|dex| dex.as_ref())
    }
}

/// Raydium AMM Program ID (mainnet)
const RAYDIUM_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
/// Raydium AMM Program ID (devnet)
const RAYDIUM_AMM_PROGRAM_ID_DEVNET: &str = "HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8";

/// Структура данных пула Raydium AMM
#[derive(Debug, Clone)]
struct RaydiumPool {
    pub pool_address: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
}

/// Реализация для Raydium
struct RaydiumDex {
    config: Config,
    rpc_client: RpcClient,
}

impl RaydiumDex {
    fn new(config: &Config) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.network.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            config: config.clone(),
            rpc_client,
        })
    }

    /// Получение адреса пула для торговой пары
    /// В реальной реализации можно использовать API Raydium или on-chain данные
    fn get_pool_address(&self, _token_a: &str, _token_b: &str) -> Result<Pubkey> {
        // Для devnet используем известные адреса пулов
        // В продакшене нужно получать через API или on-chain поиск
        let is_devnet = self.config.network.rpc_url.contains("devnet");
        
        // Пример: SOL/USDC пул на devnet
        // В реальной реализации нужно получать через Raydium API или поиск по mint адресам
        if is_devnet {
            // Заглушка: в реальности нужно получать через API
            log::warn!("Получение адреса пула через API не реализовано, используем заглушку");
            // Возвращаем случайный адрес для примера (в реальности нужно получать реальный)
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса пула")
        } else {
            // Mainnet: можно использовать известные адреса или API
            log::warn!("Получение адреса пула через API не реализовано, используем заглушку");
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса пула")
        }
    }

    /// Чтение данных пула из аккаунта
    async fn get_pool_data(&self, pool_address: &Pubkey) -> Result<RaydiumPool> {
        // Получение данных аккаунта пула
        let _account_data = self.rpc_client
            .get_account_data(pool_address)
            .context("Не удалось получить данные аккаунта пула")?;

        // Парсинг структуры пула Raydium
        // Структура может отличаться в зависимости от версии программы
        // Обычно: version (u8), status (u8), nonce (u8), orderNum (u8), 
        //         depth (u8), coinDecimals (u8), pcDecimals (u8), state (u8),
        //         resetFlag (u8), minSize (u64), volMaxCutRatio (u64),
        //         amountWaveRatio (u64), coinLotSize (u64), pcLotSize (u64),
        //         minPriceMultiplier (u64), maxPriceMultiplier (u64),
        //         systemDecimalsValue (u64), ammTargetOrders (u32),
        //         poolCoinTokenAccount (Pubkey), poolPcTokenAccount (Pubkey),
        //         coinMintAddress (Pubkey), pcMintAddress (Pubkey),
        //         lpMintAddress (Pubkey), ammOpenOrders (Pubkey),
        //         serumMarket (Pubkey), serumProgramId (Pubkey),
        //         ammTargetOrders (Pubkey), poolWithdrawQueue (Pubkey),
        //         poolTempLpTokenAccount (Pubkey), ammOwner (Pubkey),
        //         poolCoinTokenAccountPayer (Pubkey)
        
        // Упрощённая версия: читаем резервы из vault аккаунтов
        // В реальной реализации нужно парсить полную структуру
        
        // Получаем mint адреса токенов (упрощённо, в реальности из структуры пула)
        let token_a_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
        let token_b_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC
        
        // Получаем vault адреса (упрощённо)
        let token_a_vault = Pubkey::from_str("11111111111111111111111111111111")?;
        let token_b_vault = Pubkey::from_str("11111111111111111111111111111111")?;
        
        // Читаем балансы vault аккаунтов
        // В реальной реализации нужно парсить структуру пула и получать реальные vault адреса
        // Для упрощения используем заглушки
        let token_a_reserve = match self.rpc_client.get_account(&token_a_vault) {
            Ok(_account) => {
                // Парсим баланс из данных аккаунта
                // В реальности нужно использовать правильный парсинг SPL Token аккаунта
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные vault аккаунта, используем заглушку");
                0
            }
        };
        
        let token_b_reserve = match self.rpc_client.get_account(&token_b_vault) {
            Ok(_account) => {
                // Парсим баланс из данных аккаунта
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные vault аккаунта, используем заглушку");
                0
            }
        };

        Ok(RaydiumPool {
            pool_address: *pool_address,
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            token_a_reserve,
            token_b_reserve,
        })
    }

    /// Расчёт цены по формуле x*y=k (constant product)
    /// Возвращает цену: сколько quote_token за 1 base_token
    fn calculate_price(&self, pool: &RaydiumPool, base_token: &str, _quote_token: &str) -> Result<Decimal> {
        // Определяем, какой токен является base, а какой quote
        let (base_reserve, quote_reserve) = if base_token == "SOL" {
            (pool.token_a_reserve, pool.token_b_reserve)
        } else {
            (pool.token_b_reserve, pool.token_a_reserve)
        };

        if base_reserve == 0 {
            anyhow::bail!("Резерв base токена равен нулю");
        }

        // Цена = quote_reserve / base_reserve
        let price = Decimal::from(quote_reserve) / Decimal::from(base_reserve);
        Ok(price)
    }

    /// Расчёт выходного количества токенов при свопе по формуле x*y=k
    #[allow(dead_code)]
    fn calculate_swap_output(
        &self,
        pool: &RaydiumPool,
        amount_in: u64,
        is_token_a_to_b: bool,
    ) -> Result<u64> {
        let (reserve_in, reserve_out) = if is_token_a_to_b {
            (pool.token_a_reserve, pool.token_b_reserve)
        } else {
            (pool.token_b_reserve, pool.token_a_reserve)
        };

        if reserve_in == 0 || reserve_out == 0 {
            anyhow::bail!("Резерв равен нулю");
        }

        // Формула: amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        // Упрощённая версия без учёта комиссий
        let amount_out = (amount_in as u128 * reserve_out as u128) / (reserve_in as u128 + amount_in as u128);
        
        Ok(amount_out as u64)
    }

    /// Построение инструкции swap для Raydium
    fn build_swap_instruction(
        &self,
        pool: &RaydiumPool,
        user_wallet: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        is_token_a_to_b: bool,
    ) -> Result<Instruction> {
        let program_id = if self.config.network.rpc_url.contains("devnet") {
            Pubkey::from_str(RAYDIUM_AMM_PROGRAM_ID_DEVNET)?
        } else {
            Pubkey::from_str(RAYDIUM_AMM_PROGRAM_ID)?
        };

        // Построение инструкции swap
        // В реальной реализации нужно использовать правильные аккаунты и данные
        // Это упрощённая версия
        
        let accounts = vec![
            AccountMeta::new(*user_wallet, true), // user wallet (signer)
            AccountMeta::new(pool.pool_address, false), // pool
            AccountMeta::new(pool.token_a_vault, false), // token_a vault
            AccountMeta::new(pool.token_b_vault, false), // token_b vault
            AccountMeta::new(pool.token_a_mint, false), // token_a mint
            AccountMeta::new(pool.token_b_mint, false), // token_b mint
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ];

        // Данные инструкции: discriminator (8 bytes) + amount_in (8 bytes) + min_amount_out (8 bytes)
        let mut data = vec![0u8; 24];
        // В реальной реализации нужно использовать правильный discriminator для swap
        data[0..8].copy_from_slice(&[0x9a, 0x2b, 0x23, 0x1b, 0x00, 0x00, 0x00, 0x00]); // Пример discriminator
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&min_amount_out.to_le_bytes());

        Ok(Instruction {
            program_id,
            accounts,
            data,
        })
    }

    /// Отправка транзакции с retry-логикой
    async fn send_transaction_with_retry(
        &self,
        transaction: &Transaction,
        max_retries: u32,
    ) -> Result<String> {
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            match self.rpc_client.send_transaction(transaction) {
                Ok(signature) => {
                    log::info!("Транзакция отправлена успешно: {}", signature);
                    return Ok(signature.to_string());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        let delay = std::time::Duration::from_millis(100 * (attempt + 1) as u64);
                        log::warn!("Попытка {} не удалась, повтор через {:?}: {}", attempt + 1, delay, last_error.as_ref().unwrap());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Не удалось отправить транзакцию после {} попыток: {:?}",
            max_retries,
            last_error
        ))
    }
}

#[async_trait::async_trait]
impl DexInterface for RaydiumDex {
    fn name(&self) -> &str {
        "raydium"
    }

    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<Decimal> {
        log::debug!("Raydium: получение цены {}/{}", base_token, quote_token);
        
        // Получение адреса пула
        let pool_address = self.get_pool_address(base_token, quote_token)
            .context("Не удалось получить адрес пула")?;
        
        // Чтение данных пула
        let pool = self.get_pool_data(&pool_address).await
            .context("Не удалось получить данные пула")?;
        
        // Расчёт цены
        let price = self.calculate_price(&pool, base_token, quote_token)
            .context("Не удалось рассчитать цену")?;
        
        log::debug!("Raydium: цена {}/{} = {}", base_token, quote_token, price);
        Ok(price)
    }

    async fn execute_swap(
        &self,
        simulation_mode: bool,
        from_token: &str,
        to_token: &str,
        amount: Decimal,
        min_output: Decimal,
        wallet: &Wallet,
    ) -> Result<String> {
        log::info!("Raydium: выполнение свопа {} -> {} ({}), min_output: {}", 
            from_token, to_token, amount, min_output);
        
        if simulation_mode {
            log::info!("Raydium: симуляция свопа (реальная транзакция не отправляется)");
            return Ok("simulated_signature_raydium".to_string());
        }

        // Получение адреса пула
        let pool_address = self.get_pool_address(from_token, to_token)
            .context("Не удалось получить адрес пула")?;
        
        // Чтение актуальных данных пула
        let pool = self.get_pool_data(&pool_address).await
            .context("Не удалось получить данные пула")?;
        
        // Конвертация amount в lamports/token units
        // Упрощённо: предполагаем, что amount уже в правильных единицах
        let amount_in = amount.to_string().parse::<u64>()
            .context("Не удалось конвертировать amount в u64")?;
        
        let min_amount_out = min_output.to_string().parse::<u64>()
            .context("Не удалось конвертировать min_output в u64")?;
        
        // Определение направления свопа
        let is_token_a_to_b = from_token == "SOL"; // Упрощённо
        
        // Построение инструкции swap
        let swap_instruction = self.build_swap_instruction(
            &pool,
            wallet.pubkey(),
            amount_in,
            min_amount_out,
            is_token_a_to_b,
        ).context("Не удалось построить инструкцию swap")?;
        
        // Получение последнего blockhash
        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .context("Не удалось получить blockhash")?;
        
        // Создание транзакции
        let mut transaction = Transaction::new_with_payer(
            &[swap_instruction],
            Some(wallet.pubkey()),
        );
        transaction.sign(&[wallet.keypair()], recent_blockhash);
        
        // Отправка транзакции с retry
        let signature = self.send_transaction_with_retry(&transaction, 3).await
            .context("Не удалось отправить транзакцию")?;
        
        log::info!("Raydium: своп выполнен, signature: {}", signature);
        Ok(signature)
    }
}

/// Orca Whirlpools Program ID (mainnet)
const ORCA_WHIRLPOOLS_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
/// Orca Whirlpools Program ID (devnet)
const ORCA_WHIRLPOOLS_PROGRAM_ID_DEVNET: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Структура данных пула Orca Whirlpools
#[derive(Debug, Clone)]
struct OrcaWhirlpool {
    pub whirlpool_address: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub sqrt_price: u128, // sqrt price для концентрированной ликвидности
}

/// Реализация для Orca Whirlpools
struct OrcaDex {
    config: Config,
    rpc_client: RpcClient,
}

impl OrcaDex {
    fn new(config: &Config) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.network.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            config: config.clone(),
            rpc_client,
        })
    }

    /// Получение адреса Whirlpool для торговой пары
    /// В реальной реализации можно использовать Orca API или on-chain данные
    fn get_whirlpool_address(&self, _token_a: &str, _token_b: &str) -> Result<Pubkey> {
        // Для devnet используем известные адреса пулов
        // В продакшене нужно получать через API или on-chain поиск
        let is_devnet = self.config.network.rpc_url.contains("devnet");
        
        // Пример: SOL/USDC пул на devnet
        // В реальной реализации нужно получать через Orca API или поиск по mint адресам
        if is_devnet {
            // Заглушка: в реальности нужно получать через API
            log::warn!("Получение адреса Whirlpool через API не реализовано, используем заглушку");
            // Возвращаем случайный адрес для примера (в реальности нужно получать реальный)
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса Whirlpool")
        } else {
            // Mainnet: можно использовать известные адреса или API
            log::warn!("Получение адреса Whirlpool через API не реализовано, используем заглушку");
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса Whirlpool")
        }
    }

    /// Чтение данных Whirlpool из аккаунта
    async fn get_whirlpool_data(&self, whirlpool_address: &Pubkey) -> Result<OrcaWhirlpool> {
        // Получение данных аккаунта Whirlpool
        let _account_data = self.rpc_client
            .get_account_data(whirlpool_address)
            .context("Не удалось получить данные аккаунта Whirlpool")?;

        // Парсинг структуры Whirlpool
        // Структура Whirlpool (упрощённо):
        // - whirlpools_config (Pubkey)
        // - token_mint_a (Pubkey)
        // - token_vault_a (Pubkey)
        // - fee_rate (u16)
        // - tick_spacing (u16)
        // - tick_spacing_seed (u16)
        // - tick_spacing_seed2 (u16)
        // - token_mint_b (Pubkey)
        // - token_vault_b (Pubkey)
        // - fee_rate (u16)
        // - protocol_fee_rate (u16)
        // - liquidity (u128)
        // - sqrt_price (u128)
        // - tick_current_index (i32)
        // - protocol_fee_owed_a (u64)
        // - protocol_fee_owed_b (u64)
        // - tick_array_bitmap (u64[8])
        // - reward_last_updated_timestamp (u64)
        
        // Упрощённая версия: читаем резервы из vault аккаунтов
        // В реальной реализации нужно парсить полную структуру
        
        // Получаем mint адреса токенов (упрощённо, в реальности из структуры Whirlpool)
        let token_a_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
        let token_b_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC
        
        // Получаем vault адреса (упрощённо)
        let token_vault_a = Pubkey::from_str("11111111111111111111111111111111")?;
        let token_vault_b = Pubkey::from_str("11111111111111111111111111111111")?;
        
        // Читаем балансы vault аккаунтов
        // В реальной реализации нужно парсить структуру Whirlpool и получать реальные vault адреса
        // Для упрощения используем заглушки
        let token_a_reserve = match self.rpc_client.get_account(&token_vault_a) {
            Ok(_account) => {
                // Парсим баланс из данных аккаунта
                // В реальности нужно использовать правильный парсинг SPL Token аккаунта
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные vault аккаунта, используем заглушку");
                0
            }
        };
        
        let token_b_reserve = match self.rpc_client.get_account(&token_vault_b) {
            Ok(_account) => {
                // Парсим баланс из данных аккаунта
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные vault аккаунта, используем заглушку");
                0
            }
        };

        // Для Whirlpools используем sqrt_price для расчёта цены
        // Упрощённо: если резервы есть, рассчитываем sqrt_price из них
        let sqrt_price = if token_a_reserve > 0 && token_b_reserve > 0 {
            // sqrt_price = sqrt(token_b_reserve / token_a_reserve) * 2^64
            // Упрощённая версия: используем простой расчёт без integer_sqrt
            // В реальной реализации нужно использовать правильный расчёт sqrt_price
            let price_ratio = (token_b_reserve as u128 * 1_000_000_000_000_000_000) / token_a_reserve as u128;
            // Приблизительный расчёт sqrt (используем простую формулу)
            (price_ratio * (1u128 << 32)) / 1_000_000_000_000_000_000
        } else {
            0u128
        };

        Ok(OrcaWhirlpool {
            whirlpool_address: *whirlpool_address,
            token_a_mint,
            token_b_mint,
            token_vault_a,
            token_vault_b,
            token_a_reserve,
            token_b_reserve,
            sqrt_price,
        })
    }

    /// Расчёт цены из Whirlpool
    /// Возвращает цену: сколько quote_token за 1 base_token
    fn calculate_price(&self, pool: &OrcaWhirlpool, base_token: &str, _quote_token: &str) -> Result<Decimal> {
        // Определяем, какой токен является base, а какой quote
        let (base_reserve, quote_reserve) = if base_token == "SOL" {
            (pool.token_a_reserve, pool.token_b_reserve)
        } else {
            (pool.token_b_reserve, pool.token_a_reserve)
        };

        if base_reserve == 0 {
            anyhow::bail!("Резерв base токена равен нулю");
        }

        // Для Whirlpools можно использовать sqrt_price для более точного расчёта
        // Но для упрощения используем формулу из резервов
        // Цена = quote_reserve / base_reserve
        let price = Decimal::from(quote_reserve) / Decimal::from(base_reserve);
        Ok(price)
    }

    /// Расчёт выходного количества токенов при свопе
    /// Для Whirlpools используется более сложная формула с учётом концентрированной ликвидности
    /// Упрощённая версия использует формулу x*y=k
    #[allow(dead_code)]
    fn calculate_swap_output(
        &self,
        pool: &OrcaWhirlpool,
        amount_in: u64,
        is_token_a_to_b: bool,
    ) -> Result<u64> {
        let (reserve_in, reserve_out) = if is_token_a_to_b {
            (pool.token_a_reserve, pool.token_b_reserve)
        } else {
            (pool.token_b_reserve, pool.token_a_reserve)
        };

        if reserve_in == 0 || reserve_out == 0 {
            anyhow::bail!("Резерв равен нулю");
        }

        // Упрощённая формула: amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        // В реальной реализации Whirlpools использует более сложную логику с тиками и концентрированной ликвидностью
        let amount_out = (amount_in as u128 * reserve_out as u128) / (reserve_in as u128 + amount_in as u128);
        
        Ok(amount_out as u64)
    }

    /// Построение инструкции swap для Orca Whirlpools
    fn build_swap_instruction(
        &self,
        pool: &OrcaWhirlpool,
        user_wallet: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
        _is_token_a_to_b: bool,
    ) -> Result<Instruction> {
        let program_id = if self.config.network.rpc_url.contains("devnet") {
            Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID_DEVNET)?
        } else {
            Pubkey::from_str(ORCA_WHIRLPOOLS_PROGRAM_ID)?
        };

        // Построение инструкции swap для Whirlpools
        // В реальной реализации нужно использовать правильные аккаунты и данные
        // Это упрощённая версия
        
        let accounts = vec![
            AccountMeta::new(*user_wallet, true), // user wallet (signer)
            AccountMeta::new(pool.whirlpool_address, false), // whirlpool
            AccountMeta::new(pool.token_vault_a, false), // token_a vault
            AccountMeta::new(pool.token_vault_b, false), // token_b vault
            AccountMeta::new(pool.token_a_mint, false), // token_a mint
            AccountMeta::new(pool.token_b_mint, false), // token_b mint
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ];

        // Данные инструкции: discriminator (8 bytes) + amount_in (8 bytes) + min_amount_out (8 bytes)
        let mut data = vec![0u8; 24];
        // В реальной реализации нужно использовать правильный discriminator для swap Whirlpools
        // Discriminator для swap обычно: 0x02 (или другой в зависимости от версии)
        data[0..8].copy_from_slice(&[0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Пример discriminator
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&min_amount_out.to_le_bytes());

        Ok(Instruction {
            program_id,
            accounts,
            data,
        })
    }

    /// Отправка транзакции с retry-логикой
    async fn send_transaction_with_retry(
        &self,
        transaction: &Transaction,
        max_retries: u32,
    ) -> Result<String> {
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            match self.rpc_client.send_transaction(transaction) {
                Ok(signature) => {
                    log::info!("Транзакция отправлена успешно: {}", signature);
                    return Ok(signature.to_string());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        let delay = std::time::Duration::from_millis(100 * (attempt + 1) as u64);
                        log::warn!("Попытка {} не удалась, повтор через {:?}: {}", attempt + 1, delay, last_error.as_ref().unwrap());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Не удалось отправить транзакцию после {} попыток: {:?}",
            max_retries,
            last_error
        ))
    }
}

#[async_trait::async_trait]
impl DexInterface for OrcaDex {
    fn name(&self) -> &str {
        "orca"
    }

    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<Decimal> {
        log::debug!("Orca: получение цены {}/{}", base_token, quote_token);
        
        // Получение адреса Whirlpool
        let whirlpool_address = self.get_whirlpool_address(base_token, quote_token)
            .context("Не удалось получить адрес Whirlpool")?;
        
        // Чтение данных Whirlpool
        let pool = self.get_whirlpool_data(&whirlpool_address).await
            .context("Не удалось получить данные Whirlpool")?;
        
        // Расчёт цены
        let price = self.calculate_price(&pool, base_token, quote_token)
            .context("Не удалось рассчитать цену")?;
        
        log::debug!("Orca: цена {}/{} = {}", base_token, quote_token, price);
        Ok(price)
    }

    async fn execute_swap(
        &self,
        simulation_mode: bool,
        from_token: &str,
        to_token: &str,
        amount: Decimal,
        min_output: Decimal,
        wallet: &Wallet,
    ) -> Result<String> {
        log::info!("Orca: выполнение свопа {} -> {} ({}), min_output: {}", 
            from_token, to_token, amount, min_output);
        
        if simulation_mode {
            log::info!("Orca: симуляция свопа (реальная транзакция не отправляется)");
            return Ok("simulated_signature_orca".to_string());
        }

        // Получение адреса Whirlpool
        let whirlpool_address = self.get_whirlpool_address(from_token, to_token)
            .context("Не удалось получить адрес Whirlpool")?;
        
        // Чтение актуальных данных Whirlpool
        let pool = self.get_whirlpool_data(&whirlpool_address).await
            .context("Не удалось получить данные Whirlpool")?;
        
        // Конвертация amount в lamports/token units
        let amount_in = amount.to_string().parse::<u64>()
            .context("Не удалось конвертировать amount в u64")?;
        
        let min_amount_out = min_output.to_string().parse::<u64>()
            .context("Не удалось конвертировать min_output в u64")?;
        
        // Определение направления свопа
        let is_token_a_to_b = from_token == "SOL"; // Упрощённо
        
        // Построение инструкции swap
        let swap_instruction = self.build_swap_instruction(
            &pool,
            wallet.pubkey(),
            amount_in,
            min_amount_out,
            is_token_a_to_b,
        ).context("Не удалось построить инструкцию swap")?;
        
        // Получение последнего blockhash
        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .context("Не удалось получить blockhash")?;
        
        // Создание транзакции
        let mut transaction = Transaction::new_with_payer(
            &[swap_instruction],
            Some(wallet.pubkey()),
        );
        transaction.sign(&[wallet.keypair()], recent_blockhash);
        
        // Отправка транзакции с retry
        let signature = self.send_transaction_with_retry(&transaction, 3).await
            .context("Не удалось отправить транзакцию")?;
        
        log::info!("Orca: своп выполнен, signature: {}", signature);
        Ok(signature)
    }
}

/// Serum/OpenBook Program ID (mainnet)
/// Примечание: Serum был переименован в OpenBook, но старый Program ID всё ещё используется
const SERUM_PROGRAM_ID: &str = "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin";
/// OpenBook Program ID (mainnet, новая версия)
const OPENBOOK_PROGRAM_ID: &str = "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX";
/// Serum/OpenBook Program ID (devnet)
const SERUM_PROGRAM_ID_DEVNET: &str = "DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY";

/// Структура данных рынка Serum/OpenBook
#[derive(Debug, Clone)]
struct SerumMarket {
    pub market_address: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub bids: Pubkey, // Адрес аккаунта bids order book
    pub asks: Pubkey, // Адрес аккаунта asks order book
    pub best_bid_price: u64, // Лучшая цена покупки
    pub best_ask_price: u64, // Лучшая цена продажи
}

/// Реализация для Serum/OpenBook
struct SerumDex {
    config: Config,
    rpc_client: RpcClient,
}

impl SerumDex {
    fn new(config: &Config) -> Result<Self> {
        let rpc_client = RpcClient::new_with_commitment(
            config.network.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            config: config.clone(),
            rpc_client,
        })
    }

    /// Получение адреса рынка для торговой пары
    /// В реальной реализации можно использовать API или on-chain поиск
    fn get_market_address(&self, token_a: &str, token_b: &str) -> Result<Pubkey> {
        let _ = (token_a, token_b); // Пока не используется, но будет в реальной реализации
        // Для devnet используем известные адреса рынков
        // В продакшене нужно получать через API или on-chain поиск
        let is_devnet = self.config.network.rpc_url.contains("devnet");
        
        // Пример: SOL/USDC рынок
        // В реальной реализации нужно получать через Serum/OpenBook API или поиск по mint адресам
        if is_devnet {
            // Заглушка: в реальности нужно получать через API
            log::warn!("Получение адреса рынка через API не реализовано, используем заглушку");
            // Возвращаем случайный адрес для примера (в реальности нужно получать реальный)
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса рынка")
        } else {
            // Mainnet: можно использовать известные адреса или API
            log::warn!("Получение адреса рынка через API не реализовано, используем заглушку");
            Pubkey::from_str("11111111111111111111111111111111")
                .context("Ошибка создания адреса рынка")
        }
    }

    /// Чтение данных рынка из аккаунта
    async fn get_market_data(&self, market_address: &Pubkey) -> Result<SerumMarket> {
        // Получение данных аккаунта рынка
        let _account_data = self.rpc_client
            .get_account_data(market_address)
            .context("Не удалось получить данные аккаунта рынка")?;

        // Парсинг структуры рынка Serum/OpenBook
        // Структура Market (упрощённо):
        // - account_flags (u64)
        // - own_address (Pubkey)
        // - vault_signer_nonce (u64)
        // - base_mint (Pubkey)
        // - quote_mint (Pubkey)
        // - base_vault (Pubkey)
        // - quote_vault (Pubkey)
        // - fee_rates (u64)
        // - referrer_rebate_accrued (u64)
        // - bids (Pubkey) - адрес аккаунта order book для покупок
        // - asks (Pubkey) - адрес аккаунта order book для продаж
        // - event_queue (Pubkey)
        // - request_queue (Pubkey)
        
        // Упрощённая версия: используем заглушки
        // В реальной реализации нужно парсить полную структуру
        
        // Получаем mint адреса токенов (упрощённо, в реальности из структуры рынка)
        let base_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
        let quote_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC
        
        // Получаем vault адреса (упрощённо)
        let base_vault = Pubkey::from_str("11111111111111111111111111111111")?;
        let quote_vault = Pubkey::from_str("11111111111111111111111111111111")?;
        
        // Получаем адреса order book (упрощённо)
        let bids = Pubkey::from_str("11111111111111111111111111111111")?;
        let asks = Pubkey::from_str("11111111111111111111111111111111")?;
        
        // Читаем best bid/ask из order book
        // В реальной реализации нужно парсить структуру order book
        // Order book структура содержит список ордеров, отсортированных по цене
        // Best bid - самая высокая цена покупки
        // Best ask - самая низкая цена продажи
        
        // Упрощённо: используем заглушки
        // В реальной реализации нужно читать и парсить order book аккаунты
        let best_bid_price = match self.rpc_client.get_account(&bids) {
            Ok(_account) => {
                // Парсим best bid из данных order book
                // В реальности нужно использовать правильный парсинг order book структуры
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные order book bids, используем заглушку");
                0
            }
        };
        
        let best_ask_price = match self.rpc_client.get_account(&asks) {
            Ok(_account) => {
                // Парсим best ask из данных order book
                0 // Заглушка
            }
            Err(_) => {
                log::warn!("Не удалось получить данные order book asks, используем заглушку");
                0
            }
        };

        Ok(SerumMarket {
            market_address: *market_address,
            base_mint,
            quote_mint,
            base_vault,
            quote_vault,
            bids,
            asks,
            best_bid_price,
            best_ask_price,
        })
    }

    /// Расчёт цены из order book
    /// Возвращает цену: сколько quote_token за 1 base_token
    /// Использует mid price (среднее между best bid и best ask)
    fn calculate_price(&self, market: &SerumMarket, base_token: &str, quote_token: &str) -> Result<Decimal> {
        let _ = (base_token, quote_token); // Пока не используется, но может понадобиться для логики
        // Для order book DEX цена определяется из best bid/ask
        // Mid price = (best_bid + best_ask) / 2
        // Или можно использовать best ask для покупки, best bid для продажи
        
        // Проверяем, что у нас есть данные order book
        if market.best_bid_price == 0 && market.best_ask_price == 0 {
            anyhow::bail!("Order book пуст или данные не получены");
        }

        // Определяем направление: если base_token == SOL, то мы покупаем SOL за quote_token
        // Используем best ask (цена продажи) для покупки base_token
        // Или best bid (цена покупки) для продажи base_token
        
        // Для упрощения используем mid price
        let mid_price = if market.best_bid_price > 0 && market.best_ask_price > 0 {
            // Mid price = (best_bid + best_ask) / 2
            (market.best_bid_price + market.best_ask_price) / 2
        } else if market.best_ask_price > 0 {
            // Если есть только ask, используем его
            market.best_ask_price
        } else if market.best_bid_price > 0 {
            // Если есть только bid, используем его
            market.best_bid_price
        } else {
            anyhow::bail!("Нет данных для расчёта цены");
        };

        // Конвертируем цену в Decimal
        // Примечание: цена в Serum обычно хранится в специальном формате
        // Для упрощения предполагаем, что цена уже в правильных единицах
        // В реальной реализации нужно учитывать decimals токенов
        let price = Decimal::from(mid_price);
        
        // Если base_token не SOL, нужно инвертировать цену
        // (но для упрощения предполагаем, что base_token всегда первый в паре)
        Ok(price)
    }

    /// Построение инструкции для создания ордера на Serum/OpenBook
    /// Для свопа можно создать market order (ордер по рыночной цене)
    fn build_place_order_instruction(
        &self,
        market: &SerumMarket,
        user_wallet: &Pubkey,
        side: bool, // true = buy (покупка base_token), false = sell (продажа base_token)
        amount: u64,
        price: u64,
    ) -> Result<Instruction> {
        let program_id = if self.config.network.rpc_url.contains("devnet") {
            Pubkey::from_str(SERUM_PROGRAM_ID_DEVNET)?
        } else {
            // Пробуем использовать OpenBook (новый) или Serum (старый)
            Pubkey::from_str(OPENBOOK_PROGRAM_ID)
                .or_else(|_| Pubkey::from_str(SERUM_PROGRAM_ID))
                .context("Не удалось определить Program ID")?
        };

        // Построение инструкции place order для Serum/OpenBook
        // В реальной реализации нужно использовать правильные аккаунты и данные
        // Это упрощённая версия
        
        let accounts = vec![
            AccountMeta::new(*user_wallet, true), // user wallet (signer)
            AccountMeta::new(market.market_address, false), // market
            AccountMeta::new(market.bids, false), // bids order book
            AccountMeta::new(market.asks, false), // asks order book
            AccountMeta::new(market.base_vault, false), // base vault
            AccountMeta::new(market.quote_vault, false), // quote vault
            AccountMeta::new_readonly(system_program::id(), false), // system program
        ];

        // Данные инструкции: discriminator (8 bytes) + side (1 byte) + amount (8 bytes) + price (8 bytes)
        let mut data = vec![0u8; 25];
        // В реальной реализации нужно использовать правильный discriminator для place order
        // Discriminator для place order обычно: 0x03 (или другой в зависимости от версии)
        data[0..8].copy_from_slice(&[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Пример discriminator
        data[8] = if side { 1 } else { 0 }; // side: 1 = buy, 0 = sell
        data[9..17].copy_from_slice(&amount.to_le_bytes());
        data[17..25].copy_from_slice(&price.to_le_bytes());

        Ok(Instruction {
            program_id,
            accounts,
            data,
        })
    }

    /// Отправка транзакции с retry-логикой
    async fn send_transaction_with_retry(
        &self,
        transaction: &Transaction,
        max_retries: u32,
    ) -> Result<String> {
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            match self.rpc_client.send_transaction(transaction) {
                Ok(signature) => {
                    log::info!("Транзакция отправлена успешно: {}", signature);
                    return Ok(signature.to_string());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries - 1 {
                        let delay = std::time::Duration::from_millis(100 * (attempt + 1) as u64);
                        log::warn!("Попытка {} не удалась, повтор через {:?}: {}", attempt + 1, delay, last_error.as_ref().unwrap());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!(
            "Не удалось отправить транзакцию после {} попыток: {:?}",
            max_retries,
            last_error
        ))
    }
}

#[async_trait::async_trait]
impl DexInterface for SerumDex {
    fn name(&self) -> &str {
        "serum"
    }

    async fn get_price(&self, base_token: &str, quote_token: &str) -> Result<Decimal> {
        log::debug!("Serum: получение цены {}/{}", base_token, quote_token);
        
        // Получение адреса рынка
        let market_address = self.get_market_address(base_token, quote_token)
            .context("Не удалось получить адрес рынка")?;
        
        // Чтение данных рынка и order book
        let market = self.get_market_data(&market_address).await
            .context("Не удалось получить данные рынка")?;
        
        // Расчёт цены из order book
        let price = self.calculate_price(&market, base_token, quote_token)
            .context("Не удалось рассчитать цену")?;
        
        log::debug!("Serum: цена {}/{} = {}", base_token, quote_token, price);
        Ok(price)
    }

    async fn execute_swap(
        &self,
        simulation_mode: bool,
        from_token: &str,
        to_token: &str,
        amount: Decimal,
        min_output: Decimal,
        wallet: &Wallet,
    ) -> Result<String> {
        log::info!("Serum: выполнение свопа {} -> {} ({}), min_output: {}", 
            from_token, to_token, amount, min_output);
        
        if simulation_mode {
            log::info!("Serum: симуляция свопа (реальная транзакция не отправляется)");
            return Ok("simulated_signature_serum".to_string());
        }

        // Получение адреса рынка
        let market_address = self.get_market_address(from_token, to_token)
            .context("Не удалось получить адрес рынка")?;
        
        // Чтение актуальных данных рынка и order book
        let market = self.get_market_data(&market_address).await
            .context("Не удалось получить данные рынка")?;
        
        // Конвертация amount в lamports/token units
        let amount_in = amount.to_string().parse::<u64>()
            .context("Не удалось конвертировать amount в u64")?;
        
        let min_amount_out = min_output.to_string().parse::<u64>()
            .context("Не удалось конвертировать min_output в u64")?;
        
        // Определение направления свопа
        // Если from_token == SOL, то мы продаём SOL (sell), иначе покупаем (buy)
        let side = from_token != "SOL"; // true = buy base_token, false = sell base_token
        
        // Получение цены из order book для создания ордера
        // Используем best ask для покупки, best bid для продажи
        let order_price = if side {
            // Покупка: используем best ask
            if market.best_ask_price > 0 {
                market.best_ask_price
            } else {
                anyhow::bail!("Нет доступных ордеров для покупки");
            }
        } else {
            // Продажа: используем best bid
            if market.best_bid_price > 0 {
                market.best_bid_price
            } else {
                anyhow::bail!("Нет доступных ордеров для продажи");
            }
        };
        
        // Проверка минимального выхода
        // Для упрощения предполагаем, что order_price соответствует min_amount_out
        // В реальной реализации нужно рассчитывать ожидаемый выход из order book
        let _ = min_amount_out; // Пока не используется в упрощённой версии
        
        // Построение инструкции place order
        let order_instruction = self.build_place_order_instruction(
            &market,
            wallet.pubkey(),
            side,
            amount_in,
            order_price,
        ).context("Не удалось построить инструкцию place order")?;
        
        // Получение последнего blockhash
        let recent_blockhash = self.rpc_client
            .get_latest_blockhash()
            .context("Не удалось получить blockhash")?;
        
        // Создание транзакции
        let mut transaction = Transaction::new_with_payer(
            &[order_instruction],
            Some(wallet.pubkey()),
        );
        transaction.sign(&[wallet.keypair()], recent_blockhash);
        
        // Отправка транзакции с retry
        let signature = self.send_transaction_with_retry(&transaction, 3).await
            .context("Не удалось отправить транзакцию")?;
        
        log::info!("Serum: своп выполнен, signature: {}", signature);
        Ok(signature)
    }
}

