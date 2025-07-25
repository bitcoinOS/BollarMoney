import React, { useState, useEffect } from 'react';

/**
 * 钱包连接组件
 * 处理与 Unisat 钱包的连接和认证
 */
const WalletConnector = ({ onConnect, onDisconnect }) => {
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState(null);
  const [walletInstalled, setWalletInstalled] = useState(false);

  // 检查钱包是否已安装
  useEffect(() => {
    const checkWalletInstalled = () => {
      const isInstalled = typeof window.unisat !== 'undefined';
      setWalletInstalled(isInstalled);
      return isInstalled;
    };

    checkWalletInstalled();

    // 监听钱包事件
    if (walletInstalled) {
      window.unisat.on('accountsChanged', handleAccountsChanged);
      window.unisat.on('networkChanged', handleNetworkChanged);
    }

    return () => {
      // 清理事件监听
      if (walletInstalled) {
        window.unisat.removeListener('accountsChanged', handleAccountsChanged);
        window.unisat.removeListener('networkChanged', handleNetworkChanged);
      }
    };
  }, [walletInstalled]);

  // 处理账户变更
  const handleAccountsChanged = (accounts) => {
    if (accounts.length === 0) {
      // 用户断开了钱包
      onDisconnect();
    } else {
      // 账户已更改
      onConnect(accounts[0]);
    }
  };

  // 处理网络变更
  const handleNetworkChanged = (network) => {
    console.log('Bitcoin network changed:', network);
    // 可以在这里添加网络变更的处理逻辑
  };

  // 连接钱包
  const connectWallet = async () => {
    if (!walletInstalled) {
      setError('Unisat 钱包未安装。请先安装 Unisat 钱包。');
      return;
    }

    try {
      setIsConnecting(true);
      setError(null);

      // 请求连接钱包
      const accounts = await window.unisat.requestAccounts();
      
      if (accounts.length > 0) {
        // 获取网络信息
        const network = await window.unisat.getNetwork();
        
        // 获取余额信息
        const balance = await window.unisat.getBalance();
        
        // 连接成功，调用回调函数
        onConnect({
          address: accounts[0],
          network,
          balance
        });
      }
    } catch (error) {
      console.error('连接钱包失败:', error);
      setError(`连接钱包失败: ${error.message}`);
    } finally {
      setIsConnecting(false);
    }
  };

  // 断开钱包连接
  const disconnectWallet = () => {
    // Unisat 钱包没有直接的断开连接方法，我们只能在前端状态中断开
    onDisconnect();
  };

  // 安装钱包提示
  if (!walletInstalled) {
    return (
      <div className="wallet-connector">
        <div className="wallet-not-installed">
          <h3>需要安装 Unisat 钱包</h3>
          <p>请安装 Unisat 钱包以使用 Bollar Money。</p>
          <a 
            href="https://unisat.io/download" 
            target="_blank" 
            rel="noopener noreferrer"
            className="install-button"
          >
            安装 Unisat 钱包
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="wallet-connector">
      {error && <div className="error-message">{error}</div>}
      <button 
        onClick={connectWallet} 
        disabled={isConnecting}
        className="connect-button"
      >
        {isConnecting ? '连接中...' : '连接 Unisat 钱包'}
      </button>
    </div>
  );
};

export default WalletConnector;