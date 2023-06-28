// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import "v2-core/interfaces/IUniswapV2Pair.sol";
import "v2-core/interfaces/IUniswapV2Factory.sol";
import "v2-periphery/interfaces/IUniswapV2Router02.sol";
import "forge-std/console.sol";

library GeneralHelper {
    function getAmountOut(address inputToken, address outputToken, uint256 amountIn)
        public
        view
        returns (uint256 amountOut)
    {
        IUniswapV2Router02 univ2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        (uint256 reserveToken0, uint256 reserveToken1,) =
            IUniswapV2Pair(_getUniswapPair(inputToken, outputToken)).getReserves();

        uint256 reserveIn;
        uint256 reserveOut;

        if (inputToken < outputToken) {
            // inputToken is token0
            reserveIn = reserveToken0;
            reserveOut = reserveToken1;
        } else {
            // inputToken is token1
            reserveIn = reserveToken1;
            reserveOut = reserveToken0;
        }

        amountOut = univ2Router.getAmountOut(amountIn, reserveIn, reserveOut);
    }

    function getAmountIn(address inputToken, address outputToken, uint256 amountOut)
        public
        view
        returns (uint256 amountIn)
    {
        IUniswapV2Router02 univ2Router = IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);

        (uint256 reserveToken0, uint256 reserveToken1,) =
            IUniswapV2Pair(_getUniswapPair(inputToken, outputToken)).getReserves();

        uint256 reserveIn;
        uint256 reserveOut;

        if (inputToken < outputToken) {
            // inputToken is token0
            reserveIn = reserveToken0;
            reserveOut = reserveToken1;
        } else {
            // inputToken is token1
            reserveIn = reserveToken1;
            reserveOut = reserveToken0;
        }

        amountIn = univ2Router.getAmountIn(amountOut, reserveIn, reserveOut);
    }

    function _getUniswapPair(address tokenA, address tokenB) private view returns (address pair) {
        IUniswapV2Factory univ2Factory = IUniswapV2Factory(0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f);
        pair = address(IUniswapV2Pair(univ2Factory.getPair(address(tokenA), address(tokenB))));
    }
}
