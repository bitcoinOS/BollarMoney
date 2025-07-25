import React, { useState, useEffect } from 'react';
import { useWallet } from '../contexts/WalletContext';
import { api } from '../services/api';

/**
 * 用户仪表板组件
 */
const Dashboard = () => {
  const { wallet } = useWallet();
  const [positions, setPositions] = useState([]);
  const [metrics, setMetrics] = useState(null);
  const [btcPrice, setBtcPrice] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);

  // 加载用户数据
  useEffect(() => {
    const loadData = async () => {
      if (!wallet.isConnected) return;

      setIsLoading(true);
      setError(null);

      try {
        // 并行获取数据
        const [positionsData, metricsData, priceData] = await Promise.all([
          api.getUserPositions(wallet.address),
          api.getProtocolMetrics(),
          api.getBtcPrice()
        ]);

        setPositions(positionsData);
        setMetrics(metricsData);
        setBtcPrice(priceData);
      } catch (err) {
        console.error('Failed to load dashboard data:', err);
        setError('加载数据失败，请稍后再试。');
      } finally {
        setIsLoading(false);
      }
    };

    loadData();

    // 设置定时刷新
    const intervalId = setInterval(loadData, 60000); // 每分钟刷新一次

    return () => clearInterval(intervalId);
  }, [wallet.isConnected, wallet.address]);

  // 格式化 BTC 金额
  const formatBtc = (satoshis) => {
    return (satoshis / 100000000).toFixed(8);
  };

  // 格式化美元金额
  const formatUsd = (cents) => {
    return (cents / 100).toFixed(2);
  };

  // 计算健康因子颜色
  const getHealthFactorColor = (healthFactor) => {
    if (healthFactor < 120) return 'red';
    if (healthFactor < 150) return 'orange';
    return 'green';
  };

  if (!wallet.isConnected) {
    return (
      <div className="dashboard">
        <div className="not-connected">
          <h2>请先连接钱包</h2>
          <p>您需要连接比特币钱包才能查看仪表板。</p>
        </div>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="dashboard">
        <div className="loading">
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="dashboard">
        <div className="error">
          <h2>出错了</h2>
          <p>{error}</p>
          <button onClick={() => window.location.reload()}>重试</button>
        </div>
      </div>
    );
  }

  return (
    <div className="dashboard">
      <h2>用户仪表板</h2>
      
      <div className="metrics-panel">
        <div className="metric-card">
          <h3>BTC 价格</h3>
          <p className="metric-value">${formatUsd(btcPrice)}</p>
        </div>
        {metrics && (
          <>
            <div className="metric-card">
              <h3>总锁定 BTC</h3>
              <p className="metric-value">{formatBtc(metrics.total_btc_locked)} BTC</p>
            </div>
            <div className="metric-card">
              <h3>总供应 Bollar</h3>
              <p className="metric-value">{metrics.total_bollar_supply} BOLLAR</p>
            </div>
            <div className="metric-card">
              <h3>抵押率</h3>
              <p className="metric-value">{metrics.collateral_ratio}%</p>
            </div>
          </>
        )}
      </div>

      <div className="positions-panel">
        <h3>我的抵押头寸</h3>
        {positions.length === 0 ? (
          <p className="no-positions">您还没有抵押头寸。</p>
        ) : (
          <div className="positions-table">
            <table>
              <thead>
                <tr>
                  <th>头寸 ID</th>
                  <th>BTC 抵押</th>
                  <th>Bollar 债务</th>
                  <th>健康因子</th>
                  <th>创建时间</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {positions.map((position) => (
                  <tr key={position.id}>
                    <td>{position.id.substring(0, 8)}...</td>
                    <td>{formatBtc(position.btc_collateral)} BTC</td>
                    <td>{position.bollar_debt} BOLLAR</td>
                    <td>
                      <span style={{ color: getHealthFactorColor(position.health_factor) }}>
                        {(position.health_factor / 100).toFixed(2)}
                      </span>
                    </td>
                    <td>{new Date(Number(position.created_at) / 1000000).toLocaleString()}</td>
                    <td>
                      <a href={`/repay?position=${position.id}`} className="action-link">还款</a>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="actions-panel">
        <h3>快速操作</h3>
        <div className="action-buttons">
          <a href="/deposit" className="action-button">抵押 BTC & 铸造 Bollar</a>
          <a href="/repay" className="action-button">还款 Bollar & 赎回 BTC</a>
          <a href="/liquidate" className="action-button">查看可清算头寸</a>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;