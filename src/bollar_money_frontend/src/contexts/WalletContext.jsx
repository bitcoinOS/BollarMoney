import React, { createContext, useState, useContext, useEffect } from 'react';

// 创建钱包上下文
const WalletContext = createContext();

// 钱包提供者组件
export const WalletProvider = ({ children }) => {
  const [wallet, setWallet] = useState({
    isConnected: false,
    address: '',
    network: '',
    balance: {
      confirmed: 0,
      unconfirmed: 0,
      total: 0
    },
    isLoading: false,
    error: null
  });

  // 从本地存储恢复钱包状态
  useEffect(() => {
    const savedWallet = localStorage.getItem('bollar_wallet');
    if (savedWallet) {
      try {
        const parsedWallet = JSON.parse(savedWallet);
        // 如果有保存的钱包地址，尝试重新连接
        if (parsedWallet.address && typeof window.unisat !== 'undefined') {
          checkWalletConnection(parsedWallet.address);
        }
      } catch (error) {
        console.error('Failed to parse saved wallet:', error);
      }
    }
  }, []);

  // 检查钱包连接状态
  const checkWalletConnection = async (savedAddress) => {
    if (typeof window.unisat === 'undefined') return;

    try {
      setWallet(prev => ({ ...prev, isLoading: true }));
      
      // 获取当前连接的账户
      const accounts = await window.unisat.getAccounts();
      
      if (accounts.length > 0 && accounts[0] === savedAddress) {
        // 钱包已连接且地址匹配
        const network = await window.unisat.getNetwork();
        const balance = await window.unisat.getBalance();
        
        setWallet({
          isConnected: true,
          address: accounts[0],
          network,
          balance,
          isLoading: false,
          error: null
        });
        
        // 保存到本地存储
        localStorage.setItem('bollar_wallet', JSON.stringify({
          address: accounts[0],
          network
        }));
      } else {
        // 钱包未连接或地址不匹配
        clearWalletState();
      }
    } catch (error) {
      console.error('Failed to check wallet connection:', error);
      clearWalletState();
    }
  };

  // 连接钱包
  const connectWallet = async () => {
    if (typeof window.unisat === 'undefined') {
      setWallet(prev => ({
        ...prev,
        error: 'Unisat wallet not installed'
      }));
      return;
    }

    try {
      setWallet(prev => ({ ...prev, isLoading: true, error: null }));
      
      // 请求连接钱包
      const accounts = await window.unisat.requestAccounts();
      
      if (accounts.length > 0) {
        const network = await window.unisat.getNetwork();
        const balance = await window.unisat.getBalance();
        
        const walletState = {
          isConnected: true,
          address: accounts[0],
          network,
          balance,
          isLoading: false,
          error: null
        };
        
        setWallet(walletState);
        
        // 保存到本地存储
        localStorage.setItem('bollar_wallet', JSON.stringify({
          address: accounts[0],
          network
        }));
      } else {
        clearWalletState();
      }
    } catch (error) {
      console.error('Failed to connect wallet:', error);
      setWallet(prev => ({
        ...prev,
        isLoading: false,
        error: `Failed to connect wallet: ${error.message}`
      }));
    }
  };

  // 断开钱包连接
  const disconnectWallet = () => {
    clearWalletState();
  };

  // 清除钱包状态
  const clearWalletState = () => {
    setWallet({
      isConnected: false,
      address: '',
      network: '',
      balance: {
        confirmed: 0,
        unconfirmed: 0,
        total: 0
      },
      isLoading: false,
      error: null
    });
    localStorage.removeItem('bollar_wallet');
  };

  // 刷新余额
  const refreshBalance = async () => {
    if (!wallet.isConnected || typeof window.unisat === 'undefined') return;

    try {
      const balance = await window.unisat.getBalance();
      setWallet(prev => ({ ...prev, balance }));
    } catch (error) {
      console.error('Failed to refresh balance:', error);
    }
  };

  // 签名消息
  const signMessage = async (message) => {
    if (!wallet.isConnected || typeof window.unisat === 'undefined') {
      throw new Error('Wallet not connected');
    }

    try {
      return await window.unisat.signMessage(message);
    } catch (error) {
      console.error('Failed to sign message:', error);
      throw error;
    }
  };

  // 提供的上下文值
  const value = {
    wallet,
    connectWallet,
    disconnectWallet,
    refreshBalance,
    signMessage
  };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
};

// 自定义钩子，用于访问钱包上下文
export const useWallet = () => {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
};