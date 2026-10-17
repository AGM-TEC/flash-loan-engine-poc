// SPDX-License-Identifier: MIT
pragma solidity 0.8.20;

interface IERC20 {
    function transfer(address recipient, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IPool {
    function flashLoanSimple(
        address receiverAddress,
        address asset,
        uint256 amount,
        bytes calldata params,
        uint16 referralCode
    ) external;
}

interface ISwapRouter {
    struct ExactInputSingleParams {
        address tokenIn;
        address tokenOut;
        uint24 fee;
        address recipient;
        uint256 deadline;
        uint256 amountIn;
        uint256 amountOutMinimum;
        uint160 sqrtPriceLimitX96;
    }

    function exactInputSingle(ExactInputSingleParams calldata params) external payable returns (uint256 amountOut);
}

contract ArbitrageHFT {
    address public immutable owner;
    address public immutable pool;

    event ArbitrageExecuted(address indexed asset, uint256 amountBorrowed, uint256 profitGenerated);

    modifier onlyOwner() {
        require(msg.sender == owner, "ERR_UNAUTHORIZED");
        _;
    }

    modifier onlyPool() {
        require(msg.sender == pool, "ERR_INVALID_CALLER");
        _;
    }

    constructor(address _pool) {
        owner = msg.sender;
        pool = _pool;
    }

    /// @notice Point d'exécution atomique rappelé par le protocole de prêt flash
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external onlyPool returns (bool) {
        require(initiator == address(this), "ERR_INVALID_INITIATOR");

        // Décodage des paramètres d'arbitrage inter-DEX
        (
            address routerA,
            address routerB,
            address intermediaryToken,
            uint24 feeA,
            uint24 feeB,
            uint256 minProfit
        ) = abi.decode(params, (address, address, address, uint24, uint24, uint256));

        // Étape 1 : Swap initial sur le DEX A (emprunt converti en actif cible)
        IERC20(asset).approve(routerA, amount);
        
        ISwapRouter.ExactInputSingleParams memory paramsA = ISwapRouter.ExactInputSingleParams({
            tokenIn: asset,
            tokenOut: intermediaryToken,
            fee: feeA,
            recipient: address(this),
            deadline: block.timestamp,
            amountIn: amount,
            amountOutMinimum: 1,
            sqrtPriceLimitX96: 0
        });

        uint256 intermediaryBalance = ISwapRouter(routerA).exactInputSingle(paramsA);

        // Étape 2 : Swap de bouclage sur le DEX B (reconversion en actif d'emprunt)
        IERC20(intermediaryToken).approve(routerB, intermediaryBalance);

        ISwapRouter.ExactInputSingleParams memory paramsB = ISwapRouter.ExactInputSingleParams({
            tokenIn: intermediaryToken,
            tokenOut: asset,
            fee: feeB,
            recipient: address(this),
            deadline: block.timestamp,
            amountIn: intermediaryBalance,
            amountOutMinimum: 1,
            sqrtPriceLimitX96: 0
        });

        uint256 finalBalance = ISwapRouter(routerB).exactInputSingle(paramsB);
        uint256 amountOwed = amount + premium;

        // Contrôle strict de non-régression et de rentabilité nette
        require(finalBalance >= amountOwed + minProfit, "ERR_INSUFFICIENT_PROFIT");

        // Règlement du prêt flash et des frais de protocole
        IERC20(asset).approve(pool, amountOwed);

        // Extraction du profit net vers l'adresse institutionnelle / opérateur
        uint256 netProfit = finalBalance - amountOwed;
        IERC20(asset).transfer(owner, netProfit);

        emit ArbitrageExecuted(asset, amount, netProfit);

        return true;
    }

    /// @notice Déclencheur initial de la ligne de prêt flash
    function initiateArbitrage(
        address asset,
        uint256 amount,
        bytes calldata params
    ) external onlyOwner {
        IPool(pool).flashLoanSimple(address(this), asset, amount, params, 0);
    }

    receive() external payable {}
}
