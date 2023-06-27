// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/console2.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "v3-periphery/interfaces/IQuoter.sol";
import "v3-core/interfaces/IUniswapV3Pool.sol";
import "solmate/tokens/WETH.sol";

import "../src/BrainDance.sol";

/// @title BrainDanceTest
/// @author 0xmouseless
/// @notice Test suite for the BrainDance contract
contract BrainDanceTest is Test {
    address constant weth = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2;
    IUniswapV2Factory uniV2Factory;
    IUniswapV2Router02 uniV2Router;
    IQuoter uniV3Quoter;
    BrainDance brainDance;

    /// @notice Set up the testing suite
    function setUp() public {
        brainDance = new BrainDance();

        uniV2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        uniV2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);
        uniV3Quoter = IQuoter(0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6);
        WETH wrappedEther = WETH(payable(weth));

        wrappedEther.deposit{value: 10e18}();
        wrappedEther.transfer(address(brainDance), 10e18);
    }

    /// @notice Test swapping weth to usdc and back
    function testUniswapV3() public {
        address usdc = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
        address usdcWethPool = 0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640; // 500 fee pool

        // swapping 2 weth to usdc
        int256 amountIn = 2 ether;
        uint256 amountOutExpected = _quoteV3Swap(amountIn, usdcWethPool, weth, usdc);
        (uint256 amountOut,) = brainDance.calculateSwapV3(amountIn, usdcWethPool, weth, usdc);
        console2.log("swapped %d WETH for %d USDC", uint256(amountIn), amountOut);
        assertEq(
            amountOutExpected, amountOut, "WETH->USDC swap failed: received USDC deviates from expected router output."
        );

        // swapping received usdc back to weth
        amountIn = int256(amountOut);
        amountOutExpected = _quoteV3Swap(amountIn, usdcWethPool, usdc, weth);
        (amountOut,) = brainDance.calculateSwapV3(amountIn, usdcWethPool, usdc, weth);
        console2.log("swapped %d USDC for %d WETH", uint256(amountIn), amountOut);
        assertEq(
            amountOutExpected, amountOut, "USDC->WETH swap failed: received WETH deviates from expected router output."
        );
    }

    /// @notice Test swapping weth to usdc and back
    function testUniswapV2() public {
        address usdc = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48;
        address usdcWethPair = _getPairUniV2(usdc, address(weth));

        // swapping 2 weth to usdc
        uint256 amountIn = 2 ether;
        uint256 amountOutExpected = _quoteV2Swap(amountIn, usdcWethPair, weth < usdc);
        (uint256 amountOut,) = brainDance.calculateSwapV2(amountIn, usdcWethPair, weth, usdc);
        console2.log("swapped %d WETH for %d USDC", amountIn, amountOut);
        assertEq(
            amountOutExpected, amountOut, "WETH->USDC swap failed: received USDC deviates from expected router output."
        );

        // swapping received usdc back to weth
        amountIn = amountOut;
        amountOutExpected = _quoteV2Swap(amountIn, usdcWethPair, usdc < weth);
        (amountOut,) = brainDance.calculateSwapV2(amountIn, usdcWethPair, usdc, weth);
        console2.log("swapped %d USDC for %d WETH", amountIn, amountOut);
        assertEq(
            amountOutExpected, amountOut, "USDC->WETH swap failed: received WETH deviates from expected router output."
        );
    }

    /// @notice Get the deployed BrainDance bytecode (we inject this into evm instances for simulations)
    function testGetBrainDanceCode() public {
        bytes memory code = address(brainDance).code;
        emit log_bytes(code);
    }

    // HELPERS
    function _quoteV3Swap(int256 amountIn, address _pool, address tokenIn, address tokenOut)
        private
        returns (uint256 amountOut)
    {
        IUniswapV3Pool pool = IUniswapV3Pool(_pool);

        // wether tokenIn is token0 or token1
        bool zeroForOne = tokenIn < tokenOut;
        // From docs: The Q64.96 sqrt price limit. If zero for one,
        // The price cannot be less than this value after the swap.
        // If one for zero, the price cannot be greater than this value after the swap
        uint160 sqrtPriceLimitX96 = (zeroForOne ? 4295128749 : 1461446703485210103287273052203988822378723970341);

        amountOut =
            uniV3Quoter.quoteExactInputSingle(tokenIn, tokenOut, pool.fee(), uint256(amountIn), sqrtPriceLimitX96);
    }

    function _quoteV2Swap(uint256 amountIn, address pair, bool isInputToken0)
        private
        view
        returns (uint256 amountOut)
    {
        (uint256 reserveIn, uint256 reserveOut,) = IUniswapV2Pair(pair).getReserves();

        if (!isInputToken0) {
            // reserveIn is token1
            (reserveIn, reserveOut) = (reserveOut, reserveIn);
        }

        amountOut = uniV2Router.getAmountOut(amountIn, reserveIn, reserveOut);
    }

    function _getPairUniV2(address tokenA, address tokenB) private view returns (address pair) {
        pair = uniV2Factory.getPair(tokenA, tokenB);
    }
}
