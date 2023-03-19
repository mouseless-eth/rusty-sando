// SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.0;

import "../interfaces/IERC20.sol";
import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "v3-core/interfaces/IUniswapV3Pool.sol";
import "./lib/SafeMath.sol";

contract BrainDance {
    using SafeMath for uint;

    function calculateSwapV2(uint amountIn, address targetPair, address inputToken, address outputToken) external returns (uint amountOut, uint realAfterBalance){
        //////////////////////////////////////
        //            NO STRINGS            //
        //////////////////////////////////////

        // Optimistically send amountIn of inputToken to targetPair
        IERC20(inputToken).transfer(targetPair, amountIn);

        //////////////////////////////////////
        //  CALCULATING OUR EXPECTED OUTPUT //
        //////////////////////////////////////

        // Prepare variables for calculating expected amount out
        uint reserveIn;
        uint reserveOut;

        { // Avoid stack too deep error
        (uint reserve0, uint reserve1,) = IUniswapV2Pair(targetPair).getReserves();

        // sort reserves
        if (inputToken < outputToken) {
            // Token0 is equal to inputToken
            // Token1 is equal to outputToken
            reserveIn = reserve0;
            reserveOut = reserve1;
        } else {
            // Token0 is equal to outputToken
            // Token1 is equal to inputToken
            reserveIn = reserve1;
            reserveOut = reserve0;
        }
        }

        //////////////////////////////////////
        //         PERFORMING SWAP          //
        //////////////////////////////////////

        // Find the actual amountIn sent to pair (accounts for tax if any) and amountOut
        uint actualAmountIn = IERC20(inputToken).balanceOf(address(targetPair)).sub(reserveIn);
        amountOut = _getAmountOut(actualAmountIn, reserveIn, reserveOut);

        // Prepare swap variables and call pair.swap()
        (uint amount0Out, uint amount1Out) = inputToken < outputToken ? (uint(0), amountOut) : (amountOut, uint(0));
        IUniswapV2Pair(targetPair).swap(amount0Out, amount1Out, address(this), new bytes(0));

        // Find real balance after (accounts for taxed tokens)
        realAfterBalance = IERC20(outputToken).balanceOf(address(this));
    }

    function _getAmountOut(uint amountIn, uint reserveIn, uint reserveOut) internal pure returns (uint amountOut) {
        require(amountIn > 0, 'UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT');
        require(reserveIn > 0 && reserveOut > 0, 'UniswapV2Library: INSUFFICIENT_LIQUIDITY');
        uint amountInWithFee = amountIn.mul(997);
        uint numerator = amountInWithFee.mul(reserveOut);
        uint denominator = reserveIn.mul(1000).add(amountInWithFee);
        amountOut = numerator / denominator;
    }

    function calculateSwapV3(int amountIn, address targetPoolAddress, address inputToken, address outputToken) public returns (uint amountOut, uint realAfterBalance) {
        IUniswapV3Pool targetPool = IUniswapV3Pool(targetPoolAddress);
        // wether tokenIn is token0 or token1
        bool zeroForOne = inputToken < outputToken;
        // From docs: The Q64.96 sqrt price limit. If zero for one,
        // The price cannot be less than this value after the swap.
        // If one for zero, the price cannot be greater than this value after the swap
        uint160 sqrtPriceLimitX96 = (zeroForOne ? 4295128749 : 1461446703485210103287273052203988822378723970341);

        // Data used for callback
        bytes memory data = abi.encode(zeroForOne, inputToken);

        // Make swap and calc amountOut
        (int amount0, int amount1) = targetPool.swap(address(this), zeroForOne, amountIn, sqrtPriceLimitX96, data);
        amountOut = uint256(-(zeroForOne ? amount1 : amount0));

        // Find real balance after (accounts for taxed tokens)
        realAfterBalance = IERC20(outputToken).balanceOf(address(this));
    }

    function uniswapV3SwapCallback(
        int256 amount0Delta,
        int256 amount1Delta,
        bytes calldata _data
    ) external {
        require(amount0Delta > 0 || amount1Delta > 0); // swaps entirely within 0-liquidity regions are not supported
        (bool isZeroForOne, address inputToken) = abi.decode(_data, (bool, address));

        if (isZeroForOne) {
            IERC20(inputToken).transfer(msg.sender, uint(amount0Delta));
        } else {
            IERC20(inputToken).transfer(msg.sender, uint(amount1Delta));
        }
    }
}
