// deno-lint-ignore-file no-unused-vars
/**
 * Token Admin - 前端控制器
 * 注意：某些函数通过 HTML onclick 属性调用，因此需要忽略 no-unused-vars 警告
 */

const API_BASE = '/api/admin';

// 页面加载完成后加载 tokens
document.addEventListener('DOMContentLoaded', () => {
    loadTokens();
});

// 切换 IdC 字段显示
function toggleIdCFields() {
    const authType = document.getElementById('authType').value;
    const idcFields = document.getElementById('idcFields');
    idcFields.style.display = authType === 'IdC' ? 'block' : 'none';
}

// 显示消息
function showMessage(message, type = 'info') {
    const container = document.getElementById('messageContainer');
    const div = document.createElement('div');
    div.className = `message message-${type}`;
    div.textContent = message;
    container.appendChild(div);
    
    // 3秒后自动移除
    setTimeout(() => {
        div.remove();
    }, 5000);
}

// 加载 tokens 列表
async function loadTokens() {
    const listContainer = document.getElementById('tokenList');
    listContainer.innerHTML = '<p>加载中...</p>';
    
    try {
        const response = await fetch(`${API_BASE}/tokens`);
        const data = await response.json();
        
        if (data.success && data.tokens) {
            if (data.tokens.length === 0) {
                listContainer.innerHTML = '<p style="color: #64748b;">暂无 Token</p>';
                return;
            }
            
            listContainer.innerHTML = data.tokens.map((token) => `
                <div class="token-item">
                    <div class="token-info">
                        <span class="token-type">${token.auth}</span>
                        <span class="token-value">${token.refreshToken}</span>
                        ${token.description ? `<br><small style="color: #94a3b8;">${token.description}</small>` : ''}
                        ${token.clientId ? `<br><small style="color: #94a3b8;">Client ID: ${token.clientId}</small>` : ''}
                    </div>
                    <button class="btn btn-danger" onclick="deleteToken('${escapeHtml(token.refreshToken)}')">删除</button>
                </div>
            `).join('');
            
            showMessage(`已加载 ${data.tokens.length} 个 Token`, 'success');
        } else {
            listContainer.innerHTML = '<p style="color: #ef4444;">加载失败</p>';
            showMessage('加载 Token 列表失败', 'error');
        }
    } catch (error) {
        console.error('Failed to load tokens:', error);
        listContainer.innerHTML = '<p style="color: #ef4444;">加载失败</p>';
        showMessage('加载 Token 列表失败: ' + error.message, 'error');
    }
}

// 添加单个 token
async function addToken() {
    const authType = document.getElementById('authType').value;
    const refreshToken = document.getElementById('refreshToken').value.trim();
    const description = document.getElementById('description').value.trim();
    
    if (!refreshToken) {
        showMessage('请输入 Refresh Token', 'error');
        return;
    }
    
    const config = {
        auth: authType,
        refreshToken: refreshToken,
    };
    
    if (description) {
        config.description = description;
    }
    
    if (authType === 'IdC') {
        const clientId = document.getElementById('clientId').value.trim();
        const clientSecret = document.getElementById('clientSecret').value.trim();
        
        if (!clientId || !clientSecret) {
            showMessage('IdC 认证需要 Client ID 和 Client Secret', 'error');
            return;
        }
        
        config.clientId = clientId;
        config.clientSecret = clientSecret;
    }
    
    try {
        const response = await fetch(`${API_BASE}/tokens`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(config),
        });
        
        const data = await response.json();
        
        if (data.success) {
            showMessage('Token 添加成功！', 'success');
            // 清空表单
            document.getElementById('refreshToken').value = '';
            document.getElementById('description').value = '';
            document.getElementById('clientId').value = '';
            document.getElementById('clientSecret').value = '';
            // 重新加载列表
            loadTokens();
        } else {
            showMessage('添加失败: ' + data.error, 'error');
        }
    } catch (error) {
        console.error('Failed to add token:', error);
        showMessage('添加失败: ' + error.message, 'error');
    }
}

// 删除 token
async function deleteToken(refreshToken) {
    if (!confirm('确定要删除这个 Token 吗？')) {
        return;
    }
    
    try {
        const response = await fetch(`${API_BASE}/tokens`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ refreshToken }),
        });
        
        const data = await response.json();
        
        if (data.success) {
            showMessage('Token 删除成功！', 'success');
            loadTokens();
        } else {
            showMessage('删除失败: ' + data.error, 'error');
        }
    } catch (error) {
        console.error('Failed to delete token:', error);
        showMessage('删除失败: ' + error.message, 'error');
    }
}

// 批量导入 tokens
async function importTokens() {
    const input = document.getElementById('importTokensInput').value.trim();
    const mode = document.getElementById('importMode').value;
    
    if (!input) {
        showMessage('请输入要导入的 Tokens', 'error');
        return;
    }
    
    let tokens = [];
    
    try {
        // 尝试解析为 JSON
        const parsed = JSON.parse(input);
        if (Array.isArray(parsed)) {
            tokens = parsed;
        } else {
            showMessage('JSON 格式错误：应该是一个数组', 'error');
            return;
        }
    } catch {
        // 不是 JSON，尝试按行解析
        const lines = input.split('\n')
            .map(line => line.trim())
            .filter(line => line.length > 0);
        
        if (lines.length === 0) {
            showMessage('请输入至少一个 Token', 'error');
            return;
        }
        
        tokens = lines.map(line => ({
            auth: 'Social',
            refreshToken: line,
        }));
    }
    
    // 验证 tokens
    for (const token of tokens) {
        if (!token.auth || !token.refreshToken) {
            showMessage('Token 格式错误：缺少 auth 或 refreshToken', 'error');
            return;
        }
    }
    
    if (!confirm(`确定要导入 ${tokens.length} 个 Token 吗？\n模式：${mode === 'replace' ? '替换现有' : '追加到现有'}`)) {
        return;
    }
    
    try {
        const response = await fetch(`${API_BASE}/tokens/import`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ tokens, mode }),
        });
        
        const data = await response.json();
        
        if (data.success) {
            showMessage(`成功导入 ${data.count} 个 Token！`, 'success');
            document.getElementById('importTokensInput').value = '';
            loadTokens();
        } else {
            showMessage('导入失败: ' + data.error, 'error');
        }
    } catch (error) {
        console.error('Failed to import tokens:', error);
        showMessage('导入失败: ' + error.message, 'error');
    }
}

// 清空所有 tokens
async function clearAllTokens() {
    if (!confirm('⚠️ 警告：这将删除所有 Token，确定要继续吗？')) {
        return;
    }
    
    if (!confirm('再次确认：真的要清空所有 Token 吗？此操作不可恢复！')) {
        return;
    }
    
    try {
        const response = await fetch(`${API_BASE}/tokens/clear`, {
            method: 'POST',
        });
        
        const data = await response.json();
        
        if (data.success) {
            showMessage('所有 Token 已清空', 'success');
            loadTokens();
        } else {
            showMessage('清空失败: ' + data.error, 'error');
        }
    } catch (error) {
        console.error('Failed to clear tokens:', error);
        showMessage('清空失败: ' + error.message, 'error');
    }
}

// HTML 转义函数
function escapeHtml(text) {
    const map = {
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#039;'
    };
    return text.replace(/[&<>"']/g, m => map[m]);
}
