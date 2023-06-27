// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "v3-core/interfaces/IUniswapV3Pool.sol";
import "solmate/tokens/ERC20.sol";

/// @title BrainDance
/// @author 0xmouseless
/// @notice Minimal swap router used to sim V2/V3 swaps (and account for taxed tokens)
contract BrainDance {
    /**
     * @notice Performs a token swap on a v2 pool
     * @return amountOut Expected output tokens from the swap
     * @return realAfterBalance Post-swap balance, accounting for token tax
     */
    function calculateSwapV2(uint256 amountIn, address targetPair, address inputToken, address outputToken)
        external
        returns (uint256 amountOut, uint256 realAfterBalance)
    {
        //////////////////////////////////////
        //              SETUP               //
        //////////////////////////////////////

        // Optimistically send amountIn of inputToken to targetPair
        ERC20(inputToken).transfer(targetPair, amountIn);

        //////////////////////////////////////
        //  CALCULATING OUR EXPECTED OUTPUT //
        //////////////////////////////////////

        // Prepare variables for calculating expected amount out
        uint256 reserveIn;
        uint256 reserveOut;

        {
            // Avoid stack too deep error
            (uint256 reserve0, uint256 reserve1,) = IUniswapV2Pair(targetPair).getReserves();

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
        uint256 actualAmountIn = ERC20(inputToken).balanceOf(address(targetPair)) - reserveIn;
        amountOut = _getAmountOut(actualAmountIn, reserveIn, reserveOut);

        // Prepare swap variables and call pair.swap()
        (uint256 amount0Out, uint256 amount1Out) =
            inputToken < outputToken ? (uint256(0), amountOut) : (amountOut, uint256(0));
        IUniswapV2Pair(targetPair).swap(amount0Out, amount1Out, address(this), new bytes(0));

        // Find real balance after (accounts for taxed tokens)
        realAfterBalance = ERC20(outputToken).balanceOf(address(this));
    }

    /**
     * @notice Performs a token swap on a v3 pool
     * @return amountOut Expected output tokens from the swap
     * @return realAfterBalance Post-swap balance, accounting for token tax
     */
    function calculateSwapV3(int256 amountIn, address targetPoolAddress, address inputToken, address outputToken)
        public
        returns (uint256 amountOut, uint256 realAfterBalance)
    {
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
        (int256 amount0, int256 amount1) = targetPool.swap(address(this), zeroForOne, amountIn, sqrtPriceLimitX96, data);
        amountOut = uint256(-(zeroForOne ? amount1 : amount0));

        // Find real balance after (accounts for taxed tokens)
        realAfterBalance = ERC20(outputToken).balanceOf(address(this));
    }

    /**
     * @notice Post swap callback to sends amount of input token to v3 pool
     */
    function uniswapV3SwapCallback(int256 amount0Delta, int256 amount1Delta, bytes calldata _data) external {
        require(amount0Delta > 0 || amount1Delta > 0); // swaps entirely within 0-liquidity regions are not supported
        (bool isZeroForOne, address inputToken) = abi.decode(_data, (bool, address));

        if (isZeroForOne) {
            ERC20(inputToken).transfer(msg.sender, uint256(amount0Delta));
        } else {
            ERC20(inputToken).transfer(msg.sender, uint256(amount1Delta));
        }
    }

    /**
     * @notice Helper to find output amount from xy=k
     * @dev Note that fees are hardcoded to 0.3% (default for sushi and uni)
     * @return amountOut Output tokens expected from swap
     */
    function _getAmountOut(uint256 amountIn, uint256 reserveIn, uint256 reserveOut)
        internal
        pure
        returns (uint256 amountOut)
    {
        require(amountIn > 0, "UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT");
        require(reserveIn > 0 && reserveOut > 0, "UniswapV2Library: INSUFFICIENT_LIQUIDITY");
        uint256 amountInWithFee = amountIn * 997;
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = reserveIn * 1000 + amountInWithFee;
        amountOut = numerator / denominator;
    }
}
