import React, { useState } from 'react';
import { WalletProvider } from './contexts/WalletContext';
import Layout from './components/Layout';
import Dashboard from './components/Dashboard';
import DepositMint from './components/DepositMint';
import RepayWithdraw from './components/RepayWithdraw';
import Liquidation from './components/Liquidation';
import './styles/main.css';

function App() {
  const [currentPage, setCurrentPage] = useState('dashboard');

  // 简单的路由处理
  const handleNavigation = (e) => {
    e.preventDefault();
    const href = e.currentTarget.getAttribute('href');
    if (href === '/') {
      setCurrentPage('dashboard');
    } else {
      setCurrentPage(href.replace('/', ''));
    }
  };

  // 渲染当前页面
  const renderPage = () => {
    switch (currentPage) {
      case 'dashboard':
        return <Dashboard />;
      case 'deposit':
        return <DepositMint />;
      case 'repay':
        return <RepayWithdraw />;
      case 'liquidate':
        return <Liquidation />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <WalletProvider>
      <Layout>
        <div className="app-container">
          {renderPage()}
        </div>
      </Layout>
    </WalletProvider>
  );
}

export default App;