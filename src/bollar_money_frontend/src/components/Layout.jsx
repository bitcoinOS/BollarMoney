import React from 'react';
import { useWallet } from '../contexts/WalletContext';

/**
 * 应用布局组件
 */
const Layout = ({ children }) => {
  const { wallet, connectWallet, disconnectWallet } = useWallet();

  return (
    <div className="layout">
      <header className="header">
        <div className="header-content">
          <div className="logo">
            <h1>Bollar Money</h1>
          </div>
          <nav className="nav">
            <ul>
              <li><a href="/">首页</a></li>
              <li><a href="/dashboard">仪表板</a></li>
              <li><a href="/deposit">抵押 & 铸造</a></li>
              <li><a href="/repay">还款 & 赎回</a></li>
              <li><a href="/liquidate">清算</a></li>
            </ul>
          </nav>
          <div className="wallet-status">
            {wallet.isConnected ? (
              <div className="connected">
                <span className="address">{`${wallet.address.substring(0, 6)}...${wallet.address.substring(wallet.address.length - 4)}`}</span>
                <span className="balance">{`${wallet.balance.total / 100000000} BTC`}</span>
                <button onClick={disconnectWallet} className="disconnect-button">断开连接</button>
              </div>
            ) : (
              <button onClick={connectWallet} className="connect-button">连接钱包</button>
            )}
          </div>
        </div>
      </header>

      <main className="main-content">
        {children}
      </main>

      <footer className="footer">
        <div className="footer-content">
          <p>&copy; 2025 Bollar Money. All rights reserved.</p>
          <div className="footer-links">
            <a href="/about">关于我们</a>
            <a href="/terms">使用条款</a>
            <a href="/privacy">隐私政策</a>
          </div>
        </div>
      </footer>
    </div>
  );
};

export default Layout;