import React, { useState, useEffect } from 'react';
import { useWallet } from '../contexts/WalletContext';
import { api } from '../services/api';

/**
 * 还款和赎回组件
 */
const RepayWithdraw = () => {
  const { wallet } = useWallet();
  const [positions, setPositions] = useState([]);
  const [selectedPosition, setSelectedPosition] = useState(null);
  const [bollarAmount, setBollarAmount] = useState('');
  const [btcReturn, setBtcReturn] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [success, setSuccess] = useState(null);
  const [step, setStep] = useState(1); // 1: 选择头寸, 2: 输入金额, 3: 确认交易, 4: 交易结果

  // 加载用户头寸
  useEffect(() => {
    const loadPositions = async () => {
      if (!wallet.isConnected) return;

      setIsLoading(true);
      setError(null);

      try {
        const userPositions = await api.getUserPositions(wallet.address);
        setPositions(userPositions);
        
        // 检查 URL 参数中是否有指定的头寸
        const urlParams = new URLSearchParams(window.location.search);
        const positionId = urlParams.get('position');
        
        if (positionId) {
          const position = userPositions.find(p => p.id === positionId);
          if (position) {
            setSelectedPosition(position);
            setStep(2);
          }
        }
      } catch (err) {
        console.error('Failed to load positions:', err);
        setError('加载头寸失败，请稍后再试。');
      } finally {
        setIsLoading(false);
      }
    };

    loadPositions();
  }, [wallet.isConnected, wallet.address]);

  // 处理头寸选择
  const handlePositionSelect = (position) => {
    setSelectedPosition(position);
    setBollarAmount(position.bollar_debt.toString());
    setStep(2);
  };

  // 处理 Bollar 金额输入变化
  const handleBollarAmountChange = async (e) => {
    const value = e.target.value;
    setBollarAmount(value);
    
    if (!value || !selectedPosition || isNaN(parseFloat(value))) {
      setBtcReturn(0);
      return;
    }
    
    // 验证金额
    const bollarValue = parseFloat(value);
    if (bollarValue <= 0 || bollarValue > selectedPosition.bollar_debt) {
      setBtcReturn(0);
      return;
    }
    
    try {
      // 调用预还款查询
      const repayOffer = await api.preRepay(selectedPosition.id, Math.floor(bollarValue));
      setBtcReturn(repayOffer.btc_return);
    } catch (err) {
      console.error('Failed to calculate BTC return:', err);
      setError('计算可赎回 BTC 失败。');
    }
  };

  // 处理表单提交
  const handleSubmit = async (e) => {
    e.preventDefault();
    
    if (!wallet.isConnected) {
      setError('请先连接钱包。');
      return;
    }
    
    if (!selectedPosition || !bollarAmount) {
      setError('请填写所有必填字段。');
      return;
    }
    
    // 验证金额
    const bollarValue = parseFloat(bollarAmount);
    if (bollarValue <= 0 || bollarValue > selectedPosition.bollar_debt) {
      setError(`Bollar 金额必须在 1 到 ${selectedPosition.bollar_debt} 之间。`);
      return;
    }
    
    // 进入确认步骤
    setStep(3);
  };

  // 执行还款和赎回
  const executeRepay = async () => {
    setIsLoading(true);
    setError(null);
    
    try {
      // 在实际实现中，这里需要构建 PSBT 交易
      // 这里简化处理，假设已经有了签名的 PSBT
      const signedPsbt = "dummy_signed_psbt";
      
      // 调用后端执行还款和赎回
      const result = await api.executeRepay(
        selectedPosition.id,
        signedPsbt
      );
      
      setSuccess(`还款和赎回成功！交易 ID: ${result}`);
      setStep(4);
    } catch (err) {
      console.error('Failed to execute repay:', err);
      setError(`还款和赎回失败: ${err.message}`);
    } finally {
      setIsLoading(false);
    }
  };

  // 重置表单
  const resetForm = () => {
    setSelectedPosition(null);
    setBollarAmount('');
    setBtcReturn(0);
    setError(null);
    setSuccess(null);
    setStep(1);
  };

  // 格式化 BTC 金额
  const formatBtc = (satoshis) => {
    return (satoshis / 100000000).toFixed(8);
  };

  if (!wallet.isConnected) {
    return (
      <div className="repay-withdraw">
        <div className="not-connected">
          <h2>请先连接钱包</h2>
          <p>您需要连接比特币钱包才能进行还款和赎回操作。</p>
        </div>
      </div>
    );
  }

  if (isLoading && step === 1) {
    return (
      <div className="repay-withdraw">
        <div className="loading">
          <p>加载中...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="repay-withdraw">
      <h2>还款 Bollar & 赎回 BTC</h2>
      
      {error && <div className="error-message">{error}</div>}
      {success && <div className="success-message">{success}</div>}
      
      {step === 1 && (
        <div className="position-selection">
          <h3>选择头寸</h3>
          
          {positions.length === 0 ? (
            <p className="no-positions">您还没有抵押头寸。</p>
          ) : (
            <div className="positions-list">
              {positions.map((position) => (
                <div key={position.id} className="position-card" onClick={() => handlePositionSelect(position)}>
                  <div className="position-header">
                    <span className="position-id">{position.id.substring(0, 8)}...</span>
                    <span className="position-date">
                      {new Date(Number(position.created_at) / 1000000).toLocaleDateString()}
                    </span>
                  </div>
                  <div className="position-details">
                    <div className="detail-item">
                      <span className="detail-label">BTC 抵押:</span>
                      <span className="detail-value">{formatBtc(position.btc_collateral)} BTC</span>
                    </div>
                    <div className="detail-item">
                      <span className="detail-label">Bollar 债务:</span>
                      <span className="detail-value">{position.bollar_debt} BOLLAR</span>
                    </div>
                    <div className="detail-item">
                      <span className="detail-label">健康因子:</span>
                      <span className="detail-value">{(position.health_factor / 100).toFixed(2)}</span>
                    </div>
                  </div>
                  <button className="select-button">选择</button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      
      {step === 2 && selectedPosition && (
        <form onSubmit={handleSubmit} className="repay-form">
          <div className="selected-position">
            <h3>已选头寸</h3>
            <div className="position-details">
              <div className="detail-item">
                <span className="detail-label">头寸 ID:</span>
                <span className="detail-value">{selectedPosition.id.substring(0, 8)}...</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">BTC 抵押:</span>
                <span className="detail-value">{formatBtc(selectedPosition.btc_collateral)} BTC</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Bollar 债务:</span>
                <span className="detail-value">{selectedPosition.bollar_debt} BOLLAR</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">健康因子:</span>
                <span className="detail-value">{(selectedPosition.health_factor / 100).toFixed(2)}</span>
              </div>
            </div>
          </div>
          
          <div className="form-group">
            <label htmlFor="bollarAmount">还款 Bollar 金额</label>
            <input
              type="number"
              id="bollarAmount"
              value={bollarAmount}
              onChange={handleBollarAmountChange}
              placeholder="输入 Bollar 金额"
              step="1"
              min="1"
              max={selectedPosition.bollar_debt}
              required
            />
            <div className="input-actions">
              <button
                type="button"
                className="max-button"
                onClick={() => handleBollarAmountChange({ target: { value: selectedPosition.bollar_debt.toString() } })}
              >
                最大
              </button>
            </div>
          </div>
          
          <div className="info-panel">
            <div className="info-item">
              <span className="info-label">可赎回 BTC:</span>
              <span className="info-value">{formatBtc(btcReturn)} BTC</span>
            </div>
            <div className="info-item">
              <span className="info-label">剩余债务:</span>
              <span className="info-value">
                {selectedPosition.bollar_debt - parseFloat(bollarAmount || 0)} BOLLAR
              </span>
            </div>
          </div>
          
          <button type="submit" className="submit-button" disabled={isLoading || btcReturn === 0}>
            {isLoading ? '处理中...' : '继续'}
          </button>
          <button type="button" onClick={resetForm} className="cancel-button">
            返回
          </button>
        </form>
      )}
      
      {step === 3 && selectedPosition && (
        <div className="confirmation">
          <h3>确认交易</h3>
          
          <div className="confirmation-details">
            <div className="detail-item">
              <span className="detail-label">还款 Bollar:</span>
              <span className="detail-value">{bollarAmount} BOLLAR</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">赎回 BTC:</span>
              <span className="detail-value">{formatBtc(btcReturn)} BTC</span>
            </div>
            <div className="detail-item">
              <span className="detail-label">剩余债务:</span>
              <span className="detail-value">
                {selectedPosition.bollar_debt - parseFloat(bollarAmount)} BOLLAR
              </span>
            </div>
            <div className="detail-item">
              <span className="detail-label">剩余抵押:</span>
              <span className="detail-value">
                {formatBtc(selectedPosition.btc_collateral - btcReturn)} BTC
              </span>
            </div>
          </div>
          
          <div className="confirmation-actions">
            <button onClick={executeRepay} className="confirm-button" disabled={isLoading}>
              {isLoading ? '处理中...' : '确认还款和赎回'}
            </button>
            <button onClick={() => setStep(2)} className="back-button" disabled={isLoading}>
              返回
            </button>
          </div>
        </div>
      )}
      
      {step === 4 && (
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

export default RepayWithdraw;