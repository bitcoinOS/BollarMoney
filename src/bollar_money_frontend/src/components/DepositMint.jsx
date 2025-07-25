import React, { useState, useEffect } from 'react';
import { useWallet } from '../contexts/WalletContext';
import { api } from '../services/api';

/**
 * 抵押和铸造组件
 */
const DepositMint = () => {
  const { wallet } = useWallet();
  const [btcAmount, setBtcAmount] = useState('');
  const [bollarAmount, setBollarAmount] = useState('');
  const [btcPrice, setBtcPrice] = useState(0);
  const [collateralRatio, setCollateralRatio] = useState(75);
  const [maxBollarMint, setMaxBollarMint] = useState(0);
  const [poolAddress, setPoolAddress] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);
  const [step, setStep] = useState(1); // 1: 输入金额, 2: 确认交易, 3: 交易结果

  // 加载初始数据
  useEffect(() => {
    const loadInitialData = async () => {
      if (!wallet.isConnected) return;

      try {
        // 获取 BTC 价格
        const price = await api.getBtcPrice();
        setBtcPrice(price);

        // 获取协议指标以获取抵押率
        const metrics = await api.getProtocolMetrics();
        setCollateralRatio(metrics.collateral_ratio);

        // 获取池地址（假设只有一个池）
        const pools = await api.getPoolList();
        if (pools && pools.length > 0) {
          setPoolAddress(pools[0].address);
        }
      } catch (err) {
        console.error('Failed to load initial data:', err);
        setError('加载初始数据失败，请稍后再试。');
      }
    };

    loadInitialData();
  }, [wallet.isConnected]);

  // 当 BTC 金额变化时，计算可铸造的最大 Bollar 数量
  useEffect(() => {
    const calculateMaxBollar = async () => {
      if (!btcAmount || !poolAddress || isNaN(parseFloat(btcAmount))) return;

      try {
        // 将 BTC 转换为 satoshis
        const satoshis = Math.floor(parseFloat(btcAmount) * 100000000);
        
        // 调用预抵押查询
        const depositOffer = await api.preDeposit(poolAddress, satoshis);
        setMaxBollarMint(depositOffer.max_bollar_mint);
      } catch (err) {
        console.error('Failed to calculate max Bollar:', err);
        setError('计算最大可铸造 Bollar 失败。');
      }
    };

    calculateMaxBollar();
  }, [btcAmount, poolAddress, btcPrice]);

  // 处理 BTC 金额输入变化
  const handleBtcAmountChange = (e) => {
    const value = e.target.value;
    setBtcAmount(value);
    
    // 根据抵押率计算 Bollar 金额
    if (value && btcPrice && !isNaN(parseFloat(value))) {
      const btcValue = parseFloat(value) * btcPrice / 100; // 转换为美元
      const bollarValue = btcValue * (collateralRatio / 100);
      setBollarAmount(bollarValue.toFixed(2));
    } else {
      setBollarAmount('');
    }
  };

  // 处理 Bollar 金额输入变化
  const handleBollarAmountChange = (e) => {
    const value = e.target.value;
    setBollarAmount(value);
    
    // 根据抵押率计算 BTC 金额
    if (value && btcPrice && !isNaN(parseFloat(value))) {
      const bollarValue = parseFloat(value);
      const btcValue = (bollarValue / (collateralRatio / 100)) * 100 / btcPrice;
      setBtcAmount(btcValue.toFixed(8));
    } else {
      setBtcAmount('');
    }
  };

  // 处理表单提交
  const handleSubmit = async (e) => {
    e.preventDefault();
    
    if (!wallet.isConnected) {
      setError('请先连接钱包。');
      return;
    }
    
    if (!btcAmount || !bollarAmount || !poolAddress) {
      setError('请填写所有必填字段。');
      return;
    }
    
    // 验证金额
    const satoshis = Math.floor(parseFloat(btcAmount) * 100000000);
    const bollarValue = Math.floor(parseFloat(bollarAmount));
    
    if (satoshis < 10000) { // 最小 0.0001 BTC
      setError('BTC 金额太小，最小为 0.0001 BTC。');
      return;
    }
    
    if (bollarValue > maxBollarMint) {
      setError(`Bollar 金额超过最大可铸造数量 ${maxBollarMint}。`);
      return;
    }
    
    // 进入确认步骤
    setStep(2);
  };

  // 执行抵押和铸造
  const executeDeposit = async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      // 在实际实现中，这里需要构建 PSBT 交易
      // 这里简化处理，假设已经有了签名的 PSBT
      const signedPsbt = "dummy_signed_psbt";
      
      // 调用后端执行抵押和铸造
      const result = await api.executeDeposit(
        poolAddress,
        signedPsbt,
        Math.floor(parseFloat(bollarAmount))
      );
      
      setSuccess(`抵押和铸造成功！头寸 ID: ${result}`);
      setStep(3);
    } catch (err) {
      console.error('Failed to execute deposit:', err);
      setError(`抵押和铸造失败: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  // 重置表单
  const resetForm = () => {
    setBtcAmount('');
    setBollarAmount('');
    setError(null);
    setSuccess(null);
    setStep(1);
  };

  if (!wallet.isConnected) {
    return (
      <div className="deposit-mint">
        <div className="not-connected">
          <h2>请先连接钱包</h2>
          <p>您需要连接比特币钱包才能进行抵押和铸造操作。</p>
        </div>
      </div>
    );
  }

  return (
    <div className="deposit-mint">
      <h2>抵押 BTC & 铸造 Bollar</h2>
      
      {error && <div className="error-message">{error}</div>}
      {success && <div className="success-message">{success}</div>}
      
      {step === 1 && (
        <form onSubmit={handleSubmit} className="deposit-form">
          <div className="form-group">
            <label htmlFor="btcAmount">BTC 金额</label>
            <input
              type="number"
              id="btcAmount"
              value={btcAmount}
              onChange={handleBtcAmountChange}
              placeholder="输入 BTC 金额"
              step="0.00000001"
              min="0.0001"
              required
            />
            <span className="balance-info">余额: {wallet.balance.total / 100000000} BTC</span>
          </div>
          
          <div className="form-group">
            <label htmlFor="bollarAmount">Bollar 金额</label>
            <input
              type="number"
              id="bollarAmount"
              value={bollarAmount}
              onChange={handleBollarAmountChange}
              placeholder="输入 Bollar 金额"
              step="0.01"
              min="1"
              required
            />
            {maxBollarMint > 0 && (
              <span className="max-info">最大可铸造: {maxBollarMint} BOLLAR</span>
            )}
          </div>
          
          <div className="info-panel">
            <div className="info-item">
              <span className="info-label">当前 BTC 价格:</span>
              <span className="info-value">${(btcPrice / 100).toFixed(2)}</span>
            </div>
            <div className="info-item">
              <span className="info-label">抵押率:</span>
              <span className="info-value">{collateralRatio}%</span>
            </div>
            <div className="info-item">
              <span className="info-label">抵押价值:</span>
              <span className="info-value">
                ${(parseFloat(btcAmount || 0) * btcPrice / 100).toFixed(2)}
              </span>
            </div>
          </div>
          
          <button type="submit" className="submit-button" disabled={isLoading}>
            {isLoading ? '处理中...' : '继续'}
          </button>
        </form>
      )}
      
      {step === 2 && (
        <div className="confirmation">
          <h3>确认交易</h3>
          
          <div className="confirmation-details">
            <div className="detail-item">
              <span className="detail-label">抵押 BTC:</span>
              <span className="detail-value">{btcAmount} BTC</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">铸造 Bollar:</span>
              <span className="detail-value">{bollarAmount} BOLLAR</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">抵押率:</span>
              <span className="detail-value">{collateralRatio}%</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">健康因子:</span>
              <span className="detail-value">
                {((parseFloat(btcAmount) * btcPrice / 100) / parseFloat(bollarAmount)).toFixed(2)}
              </span>
            </div>
          </div>
          
          <div className="confirmation-actions">
            <button onClick={executeDeposit} className="confirm-button" disabled={isLoading}>
              {isLoading ? '处理中...' : '确认抵押和铸造'}
            </button>
            <button onClick={resetForm} className="cancel-button" disabled={isLoading}>
              取消
            </button>
          </div>
        </div>
      )}
      
      {step === 3 && (
        <div className="result">
          <h3>交易结果</h3>
          
          {success && (
            <div className="success-details">
              <p>{success}</p>
              <button onClick={resetForm} className="new-transaction-button">
                新建交易
              </button>
              <a href="/dashboard" className="dashboard-link">
                查看仪表板
              </a>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default DepositMint;