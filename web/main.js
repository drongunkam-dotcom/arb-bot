// Web dashboard for arb-bot
const API_URL = 'http://localhost:8080';

async function fetchStats() {
    try {
        const response = await fetch(`${API_URL}/stats`);
        const data = await response.json();
        
        document.getElementById('balance').textContent = `${data.balance} ETH`;
        document.getElementById('profit').textContent = `${data.profit} ETH`;
        document.getElementById('trades').textContent = data.trades;
    } catch (error) {
        console.error('Error fetching stats:', error);
    }
}

async function fetchOpportunities() {
    try {
        const response = await fetch(`${API_URL}/opportunities`);
        const opportunities = await response.json();
        
        const list = document.getElementById('opportunities-list');
        list.innerHTML = '';
        
        opportunities.forEach(opp => {
            const div = document.createElement('div');
            div.className = 'log-entry';
            div.innerHTML = `
                <strong>${opp.dex_in} → ${opp.dex_out}</strong><br>
                Прибыль: ${opp.profit_percentage}% (${opp.expected_profit} ETH)
            `;
            list.appendChild(div);
        });
    } catch (error) {
        console.error('Error fetching opportunities:', error);
    }
}

async function fetchLogs() {
    try {
        const response = await fetch(`${API_URL}/logs`);
        const logs = await response.json();
        
        const container = document.getElementById('logs-container');
        container.innerHTML = '';
        
        logs.forEach(log => {
            const div = document.createElement('div');
            div.className = 'log-entry';
            div.textContent = `[${log.timestamp}] ${log.message}`;
            container.appendChild(div);
        });
    } catch (error) {
        console.error('Error fetching logs:', error);
    }
}

// Update dashboard every 5 seconds
setInterval(() => {
    fetchStats();
    fetchOpportunities();
    fetchLogs();
}, 5000);

// Initial load
fetchStats();
fetchOpportunities();
fetchLogs();







