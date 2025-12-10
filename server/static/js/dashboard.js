/**
 * Token Dashboard - 前端控制器
 * 基于模块化设计，遵循单一职责原则
 */

class TokenDashboard {
    constructor() {
        this.autoRefreshInterval = null;
        this.isAutoRefreshEnabled = false;
        this.apiBaseUrl = '/api';
        
        this.init();
    }

    /**
     * 初始化Dashboard
     */
    init() {
        this.bindEvents();
        this.refreshTokens();
    }

    /**
     * 绑定事件处理器 (DRY原则)
     */
    bindEvents() {
        // 手动刷新按钮
        const refreshBtn = document.querySelector('.refresh-btn');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => this.refreshTokens());
        }

        // 自动刷新开关
        const switchEl = document.querySelector('.switch');
        if (switchEl) {
            switchEl.addEventListener('click', () => this.toggleAutoRefresh());
        }
    }

    /**
     * 获取Token数据 - 简单直接 (KISS原则)
     */
    async refreshTokens() {
        const tbody = document.getElementById('tokenTableBody');
        this.showLoading(tbody, '正在刷新Token数据...');
        
        try {
            const response = await fetch(`${this.apiBaseUrl}/tokens`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            
            const data = await response.json();
            this.updateTokenTable(data);
            this.updateStatusBar(data);
            this.updateLastUpdateTime();
            
        } catch (error) {
            console.error('刷新Token数据失败:', error);
            this.showError(tbody, `加载失败: ${error.message}`);
        }
    }

    /**
     * 更新Token表格 (OCP原则 - 易于扩展新字段)
     */
    updateTokenTable(data) {
        const tbody = document.getElementById('tokenTableBody');
        
        if (!data.tokens || data.tokens.length === 0) {
            this.showError(tbody, '暂无Token数据');
            return;
        }
        
        const rows = data.tokens.map(token => this.createTokenRow(token)).join('');
        tbody.innerHTML = rows;
    }

    /**
     * 创建单个Token行 (SRP原则)
     */
    createTokenRow(token) {
        const statusClass = this.getStatusClass(token);
        const statusText = this.getStatusText(token);
        
        return `
            <tr>
                <td>${token.user_email || 'unknown'}</td>
                <td><span class="token-preview">${token.token_preview || 'N/A'}</span></td>
                <td>${token.auth_type || 'social'}</td>
                <td>${token.remaining_usage || 0}</td>
                <td>${this.formatDateTime(token.expires_at)}</td>
                <td>${this.formatDateTime(token.last_used)}</td>
                <td><span class="status-badge ${statusClass}">${statusText}</span></td>
            </tr>
        `;
    }

    /**
     * 更新状态栏 (SRP原则)
     */
    updateStatusBar(data) {
        this.updateElement('totalTokens', data.total_tokens || 0);
        this.updateElement('activeTokens', data.active_tokens || 0);
    }

    /**
     * 更新最后更新时间
     */
    updateLastUpdateTime() {
        const now = new Date();
        const timeStr = now.toLocaleTimeString('zh-CN', { hour12: false });
        this.updateElement('lastUpdate', timeStr);
    }

    /**
     * 切换自动刷新 (ISP原则 - 接口隔离)
     */
    toggleAutoRefresh() {
        const switchEl = document.querySelector('.switch');
        
        if (this.isAutoRefreshEnabled) {
            this.stopAutoRefresh();
            switchEl.classList.remove('active');
        } else {
            this.startAutoRefresh();
            switchEl.classList.add('active');
        }
    }

    /**
     * 启动自动刷新
     */
    startAutoRefresh() {
        this.autoRefreshInterval = setInterval(() => this.refreshTokens(), 30000);
        this.isAutoRefreshEnabled = true;
    }

    /**
     * 停止自动刷新
     */
    stopAutoRefresh() {
        if (this.autoRefreshInterval) {
            clearInterval(this.autoRefreshInterval);
            this.autoRefreshInterval = null;
        }
        this.isAutoRefreshEnabled = false;
    }

    /**
     * 工具方法 - 状态判断 (KISS原则)
     */
    getStatusClass(token) {
        if (new Date(token.expires_at) < new Date()) {
            return 'status-expired';
        }
        const remaining = token.remaining_usage || 0;
        if (remaining === 0) return 'status-exhausted';
        if (remaining <= 5) return 'status-low';
        return 'status-active';
    }

    getStatusText(token) {
        if (new Date(token.expires_at) < new Date()) {
            return '已过期';
        }
        const remaining = token.remaining_usage || 0;
        if (remaining === 0) return '已耗尽';
        if (remaining <= 5) return '即将耗尽';
        return '正常';
    }

    /**
     * 工具方法 - 日期格式化 (DRY原则)
     */
    formatDateTime(dateStr) {
        if (!dateStr) return '-';
        
        try {
            const date = new Date(dateStr);
            if (isNaN(date.getTime())) return '-';
            
            return date.toLocaleString('zh-CN', {
                year: 'numeric',
                month: '2-digit',
                day: '2-digit',
                hour: '2-digit',
                minute: '2-digit',
                hour12: false
            });
        } catch {
            return '-';
        }
    }

    /**
     * UI工具方法 (KISS原则)
     */
    updateElement(id, content) {
        const element = document.getElementById(id);
        if (element) element.textContent = content;
    }

    showLoading(container, message) {
        container.innerHTML = `
            <tr>
                <td colspan="7" class="loading">
                    <div class="spinner"></div>
                    ${message}
                </td>
            </tr>
        `;
    }

    showError(container, message) {
        container.innerHTML = `
            <tr>
                <td colspan="7" class="error">
                    ${message}
                </td>
            </tr>
        `;
    }
}

// DOM加载完成后初始化 (依赖注入原则)
document.addEventListener('DOMContentLoaded', () => {
    new TokenDashboard();
});