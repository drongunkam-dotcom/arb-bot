// Arb Bot Dashboard - Fortnite Style
// API и WebSocket клиент

class ArbBotDashboard {
    constructor() {
        this.apiBase = '/api';
        this.wsUpdates = null;
        this.wsLogs = null;
        this.profitChart = null;
        this.profitData = {
            labels: [],
            values: []
        };
        this.updateInterval = null;
        this.authHeader = this.getAuthHeader();
        
        this.init();
    }

    init() {
        this.setupEventListeners();
        this.loadInitialData();
        this.connectWebSockets();
        this.setupChart();
        this.startAutoRefresh();
    }

    getAuthHeader() {
        // Basic Auth из localStorage или prompt
        const stored = localStorage.getItem('arb_bot_auth');
        if (stored) {
            return 'Basic ' + stored;
        }
        
        // Запрашиваем у пользователя
        const username = prompt('Введите имя пользователя:');
        const password = prompt('Введите пароль:');
        if (username && password) {
            const auth = btoa(`${username}:${password}`);
            localStorage.setItem('arb_bot_auth', auth);
            return 'Basic ' + auth;
        }
        
        return null;
    }

    async apiCall(endpoint, options = {}) {
        const url = `${this.apiBase}${endpoint}`;
        const headers = {
            'Content-Type': 'application/json',
            ...options.headers
        };
        
        if (this.authHeader) {
            headers['Authorization'] = this.authHeader;
        }

        try {
            const response = await fetch(url, {
                ...options,
                headers
            });

            if (response.status === 401) {
                // Неавторизован - очищаем и запрашиваем снова
                localStorage.removeItem('arb_bot_auth');
                this.authHeader = this.getAuthHeader();
                return this.apiCall(endpoint, options);
            }

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error(`API Error (${endpoint}):`, error);
            throw error;
        }
    }

    async loadInitialData() {
        await Promise.all([
            this.updateStatus(),
            this.updateBalance(),
            this.updateMetrics(),
            this.updateOpportunities(),
            this.updateHistory(),
            this.updateConfig()
        ]);
    }

    async updateStatus() {
        try {
            const data = await this.apiCall('/status');
            const statusEl = document.getElementById('botStatus');
            const statusIndicator = document.querySelector('.status-dot');
            const statusText = document.querySelector('.status-text');
            const simMode = document.getElementById('simulationMode');
            const uptime = document.getElementById('uptime');

            statusEl.textContent = data.status === 'running' ? '🟢 АКТИВЕН' : 
                                  data.status === 'stopped' ? '🔴 ОСТАНОВЛЕН' : 
                                  '⚠️ ОШИБКА';

            if (data.status === 'running') {
                statusIndicator.classList.add('active');
                statusText.textContent = 'Активен';
            } else {
                statusIndicator.classList.remove('active');
                statusText.textContent = data.status === 'stopped' ? 'Остановлен' : 'Ошибка';
            }

            simMode.textContent = data.simulation_mode ? 'Симуляция' : 'Реальный';
            uptime.textContent = this.formatUptime(data.uptime_seconds);
        } catch (error) {
            console.error('Ошибка обновления статуса:', error);
        }
    }

    async updateBalance() {
        try {
            const data = await this.apiCall('/balance');
            document.getElementById('solBalance').textContent = parseFloat(data.sol_balance).toFixed(4) + ' SOL';
            document.getElementById('usdEquivalent').textContent = '$' + parseFloat(data.usd_equivalent).toFixed(2);
        } catch (error) {
            console.error('Ошибка обновления баланса:', error);
        }
    }

    async updateMetrics() {
        try {
            const data = await this.apiCall('/metrics');
            document.getElementById('totalTrades').textContent = data.total_trades;
            document.getElementById('successfulTrades').textContent = data.successful_trades;
            document.getElementById('failedTrades').textContent = data.failed_trades;
            document.getElementById('avgProfitPercent').textContent = parseFloat(data.average_profit_percent).toFixed(2) + '%';
            document.getElementById('totalProfitSol').textContent = parseFloat(data.total_profit_sol).toFixed(4) + ' SOL';
            document.getElementById('totalProfitUsd').textContent = '$' + parseFloat(data.total_profit_usd).toFixed(2);

            // Добавляем точку в график
            if (data.last_trade_timestamp) {
                const timestamp = new Date(data.last_trade_timestamp);
                this.profitData.labels.push(timestamp.toLocaleTimeString());
                this.profitData.values.push(parseFloat(data.total_profit_sol));
                
                // Ограничиваем до 50 точек
                if (this.profitData.labels.length > 50) {
                    this.profitData.labels.shift();
                    this.profitData.values.shift();
                }
                
                this.updateChart();
            }
        } catch (error) {
            console.error('Ошибка обновления метрик:', error);
        }
    }

    async updateOpportunities() {
        try {
            const data = await this.apiCall('/opportunities?limit=20');
            const tbody = document.getElementById('opportunitiesBody');
            
            if (data.opportunities.length === 0) {
                tbody.innerHTML = '<tr><td colspan="8" class="loading">Нет доступных возможностей</td></tr>';
                return;
            }

            tbody.innerHTML = data.opportunities.map(opp => {
                const profit = parseFloat(opp.profit_percent_after_fees);
                const profitClass = profit > 0 ? 'profit-positive' : 'profit-negative';
                
                return `
                    <tr>
                        <td><strong>${opp.from_dex}</strong></td>
                        <td><strong>${opp.to_dex}</strong></td>
                        <td>${opp.base_token}/${opp.quote_token}</td>
                        <td>${parseFloat(opp.buy_price).toFixed(6)}</td>
                        <td>${parseFloat(opp.sell_price).toFixed(6)}</td>
                        <td class="${profitClass}">${profit.toFixed(2)}%</td>
                        <td class="${profitClass}">${parseFloat(opp.profit_percent_after_fees).toFixed(2)}%</td>
                        <td>${parseFloat(opp.trade_amount).toFixed(4)}</td>
                    </tr>
                `;
            }).join('');
        } catch (error) {
            console.error('Ошибка обновления возможностей:', error);
            document.getElementById('opportunitiesBody').innerHTML = 
                '<tr><td colspan="8" class="loading">Ошибка загрузки</td></tr>';
        }
    }

    async updateHistory() {
        try {
            const statusFilter = document.getElementById('filterStatus').value;
            const dexFilter = document.getElementById('filterDex').value;
            
            let url = '/history?limit=50';
            if (statusFilter) url += `&status=${statusFilter}`;
            if (dexFilter) url += `&from_dex=${dexFilter}`;
            
            const data = await this.apiCall(url);
            const tbody = document.getElementById('historyBody');
            
            if (data.trades.length === 0) {
                tbody.innerHTML = '<tr><td colspan="9" class="loading">Нет истории сделок</td></tr>';
                return;
            }

            tbody.innerHTML = data.trades.map(trade => {
                const statusClass = `status-${trade.status}`;
                const profit = parseFloat(trade.profit_percent);
                const profitClass = profit > 0 ? 'profit-positive' : 'profit-negative';
                const timestamp = new Date(trade.timestamp);
                
                return `
                    <tr>
                        <td>${timestamp.toLocaleString()}</td>
                        <td><strong>${trade.from_dex}</strong></td>
                        <td><strong>${trade.to_dex}</strong></td>
                        <td>${trade.base_token}/${trade.quote_token}</td>
                        <td>${parseFloat(trade.amount).toFixed(4)}</td>
                        <td class="${profitClass}">${profit.toFixed(2)}%</td>
                        <td class="${profitClass}">${parseFloat(trade.profit_sol).toFixed(4)} SOL</td>
                        <td class="${statusClass}">${this.getStatusText(trade.status)}</td>
                        <td>${trade.tx_signature ? 
                            `<a href="https://solscan.io/tx/${trade.tx_signature}" target="_blank" style="color: var(--fortnite-blue);">🔗</a>` : 
                            '-'}</td>
                    </tr>
                `;
            }).join('');
        } catch (error) {
            console.error('Ошибка обновления истории:', error);
            document.getElementById('historyBody').innerHTML = 
                '<tr><td colspan="9" class="loading">Ошибка загрузки</td></tr>';
        }
    }

    async updateConfig() {
        try {
            const data = await this.apiCall('/config');
            const container = document.getElementById('configContent');
            
            container.innerHTML = `
                <div class="config-item">
                    <h3>🌐 Сеть</h3>
                    <p><strong>RPC URL:</strong> ${data.network.rpc_url}</p>
                    <p><strong>Commitment:</strong> ${data.network.commitment}</p>
                </div>
                <div class="config-item">
                    <h3>⚡ Арбитраж</h3>
                    <p><strong>Мин. прибыль:</strong> ${data.arbitrage.min_profit_percent}%</p>
                    <p><strong>Макс. объём:</strong> ${data.arbitrage.max_trade_amount_sol} SOL</p>
                    <p><strong>Slippage:</strong> ${data.arbitrage.slippage_tolerance}%</p>
                </div>
                <div class="config-item">
                    <h3>💱 DEX</h3>
                    <p><strong>Активные:</strong> ${data.dex.enabled_dexes.join(', ')}</p>
                    <p><strong>Пары:</strong> ${data.dex.trading_pairs.length} пар</p>
                </div>
                <div class="config-item">
                    <h3>📊 Мониторинг</h3>
                    <p><strong>Интервал:</strong> ${data.monitoring.check_interval_ms}ms</p>
                    <p><strong>Уровень логов:</strong> ${data.monitoring.log_level}</p>
                </div>
                <div class="config-item">
                    <h3>🔒 Безопасность</h3>
                    <p><strong>Режим симуляции:</strong> ${data.safety.simulation_mode ? 'Да' : 'Нет'}</p>
                    <p><strong>Макс. ошибок:</strong> ${data.safety.max_consecutive_failures}</p>
                    <p><strong>Мин. баланс:</strong> ${data.safety.min_balance_sol} SOL</p>
                </div>
            `;
        } catch (error) {
            console.error('Ошибка обновления конфигурации:', error);
        }
    }

    setupChart() {
        const ctx = document.getElementById('profitChart').getContext('2d');
        this.profitChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels: this.profitData.labels,
                datasets: [{
                    label: 'Прибыль (SOL)',
                    data: this.profitData.values,
                    borderColor: '#00D4FF',
                    backgroundColor: 'rgba(0, 212, 255, 0.1)',
                    borderWidth: 3,
                    fill: true,
                    tension: 0.4,
                    pointRadius: 4,
                    pointHoverRadius: 6,
                    pointBackgroundColor: '#00D4FF',
                    pointBorderColor: '#8B5CF6',
                    pointBorderWidth: 2
                }]
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        labels: {
                            color: '#00D4FF',
                            font: {
                                family: 'Orbitron',
                                size: 14,
                                weight: 'bold'
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        ticks: {
                            color: '#00D4FF'
                        },
                        grid: {
                            color: 'rgba(0, 212, 255, 0.1)'
                        }
                    },
                    y: {
                        ticks: {
                            color: '#00D4FF'
                        },
                        grid: {
                            color: 'rgba(0, 212, 255, 0.1)'
                        }
                    }
                }
            }
        });
    }

    updateChart() {
        if (this.profitChart) {
            this.profitChart.data.labels = this.profitData.labels;
            this.profitChart.data.datasets[0].data = this.profitData.values;
            this.profitChart.update('none');
        }
    }

    connectWebSockets() {
        // WebSocket для обновлений
        this.connectUpdatesWS();
        // WebSocket для логов
        this.connectLogsWS();
    }

    connectUpdatesWS() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws/updates`;
        
        try {
            this.wsUpdates = new WebSocket(wsUrl);
            
            this.wsUpdates.onopen = () => {
                console.log('WebSocket (updates) подключен');
            };
            
            this.wsUpdates.onmessage = (event) => {
                const data = JSON.parse(event.data);
                // Обновляем данные при получении сообщения
                this.updateStatus();
                this.updateBalance();
                this.updateMetrics();
                this.updateOpportunities();
            };
            
            this.wsUpdates.onerror = (error) => {
                console.error('WebSocket (updates) ошибка:', error);
            };
            
            this.wsUpdates.onclose = () => {
                console.log('WebSocket (updates) отключен, переподключение через 5 сек...');
                setTimeout(() => this.connectUpdatesWS(), 5000);
            };
        } catch (error) {
            console.error('Ошибка подключения WebSocket (updates):', error);
        }
    }

    connectLogsWS() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws/logs`;
        
        try {
            this.wsLogs = new WebSocket(wsUrl);
            
            this.wsLogs.onopen = () => {
                console.log('WebSocket (logs) подключен');
            };
            
            this.wsLogs.onmessage = (event) => {
                const log = JSON.parse(event.data);
                this.addLogEntry(log);
            };
            
            this.wsLogs.onerror = (error) => {
                console.error('WebSocket (logs) ошибка:', error);
            };
            
            this.wsLogs.onclose = () => {
                console.log('WebSocket (logs) отключен, переподключение через 5 сек...');
                setTimeout(() => this.connectLogsWS(), 5000);
            };
        } catch (error) {
            console.error('Ошибка подключения WebSocket (logs):', error);
        }
    }

    addLogEntry(log) {
        const container = document.getElementById('logsContainer');
        const entry = document.createElement('div');
        entry.className = `log-entry log-${log.level || 'info'}`;
        
        const timestamp = new Date(log.timestamp || Date.now()).toLocaleTimeString();
        entry.textContent = `[${timestamp}] ${log.message || log}`;
        
        container.appendChild(entry);
        
        // Прокрутка вниз
        container.scrollTop = container.scrollHeight;
        
        // Ограничение количества логов (оставляем последние 100)
        while (container.children.length > 100) {
            container.removeChild(container.firstChild);
        }
    }

    setupEventListeners() {
        // Кнопки управления
        document.getElementById('btnStart').addEventListener('click', () => this.controlBot('start'));
        document.getElementById('btnStop').addEventListener('click', () => this.controlBot('stop'));
        
        // Кнопки обновления
        document.getElementById('btnRefreshOpportunities').addEventListener('click', () => this.updateOpportunities());
        document.getElementById('btnRefreshHistory').addEventListener('click', () => this.updateHistory());
        
        // Фильтры
        document.getElementById('filterStatus').addEventListener('change', () => this.updateHistory());
        document.getElementById('filterDex').addEventListener('change', () => this.updateHistory());
        
        // Очистка логов
        document.getElementById('btnClearLogs').addEventListener('click', () => {
            document.getElementById('logsContainer').innerHTML = '';
        });
    }

    async controlBot(action) {
        try {
            const data = await this.apiCall(`/control/${action}`, {
                method: 'POST'
            });
            
            alert(data.message);
            await this.updateStatus();
        } catch (error) {
            alert(`Ошибка: ${error.message}`);
        }
    }

    startAutoRefresh() {
        // Обновление каждые 5 секунд
        this.updateInterval = setInterval(() => {
            this.updateStatus();
            this.updateBalance();
            this.updateMetrics();
            this.updateOpportunities();
        }, 5000);
    }

    formatUptime(seconds) {
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        
        if (days > 0) return `${days}д ${hours}ч ${minutes}м`;
        if (hours > 0) return `${hours}ч ${minutes}м ${secs}с`;
        if (minutes > 0) return `${minutes}м ${secs}с`;
        return `${secs}с`;
    }

    getStatusText(status) {
        const statusMap = {
            'success': '✅ Успешно',
            'failed': '❌ Ошибка',
            'simulated': '🎮 Симуляция'
        };
        return statusMap[status] || status;
    }
}

// Инициализация при загрузке страницы
document.addEventListener('DOMContentLoaded', () => {
    window.dashboard = new ArbBotDashboard();
});

