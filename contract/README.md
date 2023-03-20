# Contract ![license](https://img.shields.io/badge/License-MIT-green.svg?label=license)

Gas optimized sando contract written in Huff to make use unconventional gas optimizations. 

> Why not Yul? Yul does not give access to the stack or jump instructions. 

## Gas Optimizations

### JUMPDEST Function Sig
Instead of reserving 4 bytes for a function selector, store a JUMPDEST in the first byte of calldata and jump to it at the beginning of execution. Doing so allows us to jump to the code range 0x00-0xFF, fill range with place holder JUMPDEST that point to location of function body. 

Example:
```as
#define macro MAIN() = takes (0) returns (0) {
    // extract function selector (JUMPDEST encoding)
    returndatasize                              // [0x00]
    calldataload                                // [calldata]
    returndatasize                              // [0x00, calldata]
    byte                                        // [jumplabel]
    jump                                        // []
```

> **Note**
> JUMPDEST 0xfa is reserved to handle [UniswapV3 callback](https://docs.uniswap.org/contracts/v3/reference/core/interfaces/callback/IUniswapV3SwapCallback).

### Encoding WETH Value Using tx.value
When dealing with WETH amounts, the amount is encoded by first dividing the value by 100000, and setting the divided value as `tx.value` when calling the contract. The contract then multiplies `tx.value` by 100000 to get the original amount. 

> the last 5 digits of the original value are lost after encoding, however it is a small amount of wei that we can ignore it.
Example:
```as
    // setup calldata for swap(wethOut, 0, address(this), "")
    [V2_Swap_Sig] 0x00 mstore                   
    0x0186a0 callvalue mul 0x04 mstore          // original weth value is decoded here by doing 100000 * callvalue    
    0x00 0x24 mstore                   
    address 0x44 mstore                         
    0x80 0x64 mstore                     
```

### Encoding Other Token Value Using 5 Bytes Of Calldata
When dealing with the other token amount, the values can range significantlly depending on token decimal and total supply. To account for full range, we encode by fitting the value into 4 bytes of calldata plus a byte shift. To decode, we byteshift the 4bytes to the left. 

We use byteshifts instead of bitshifts because we perform a byteshift by storing the 4bytes in memory N bytes to the left of its memory slot. 

However, instead of encoding the byteshift into our calldata, we encode the offset in memory such that when the 4bytes are stored, it will be N bytes from the left of its storage slot.

> **Note** 
> Free alfa: Might be able to optimize contract by eliminating unnecessary [memory expansions](https://www.evm.codes/about#memoryexpansion) by changing order that params are stored in memory. I did not account for this when writing the contract. 

### Hardcoded values
Weth address is hardcoded into the contract and there are individual methods to handle when Weth is token0 or token1. 

### Encode Packed
All calldata is encoded by packing the values together. 

## Interface

| JUMPDEST  | Function Name |
| :-------------: | :------------- |
| 0x06  | V2 Swap, Weth is Token0 and Output  |
| 0x0B  | V2 Swap, Weth is Token0 and Input  |
| 0x10  | V2 Swap, Weth is Token1 and Output  |
| 0x15  | V2 Swap, Weth is Token1 and Input |
| 0x1A  | V3 Swap, Weth is Token1 and Output, Big Encoding |
| 0x1F  | V3 Swap, Weth is Token0 and Output, Big Encoding  |
| 0x24  | V3 Swap, Weth is Token1 and Output, Small Encoding  |
| 0x29  | V3 Swap, Weth is Token0 and Output, Small Encoding |
| 0x2E  | V3 Swap, Weth is Token0 and Input  |
| 0x33  | V3 Swap, Weth is Token1 and Input  |
| 0x38  | Seppuku (self-destruct)  |
| 0x3D  | Recover Eth  |
| 0x42  | Recover Weth  |
| 0xFA  | UniswapV3 Callback  |


## Calldata Encoding 
### Uniswap V2 Calldata Encoding Format

#### When Weth is input (0x0B, 0x15)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 1 | Where to store AmountOut  |
| 4 | AmountOut  |

#### When Weth is output (0x06, 0x10)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 1 | Where to store AmountIn  |
| 4 | AmountIn  |

### Uniswap V3 Calldata Encoding Format

#### When Weth is input (0x2E, 0x33)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 32 | PairInitHash  | 
> PairInitHash used to verify msg.sender is pool in callback

#### When Weth is output small (0x2E, 0x33)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 6 | AmountIn  | 
| 32 | PairInitHash  | 
> Small encoding when AmountIn < 10^12

#### When Weth is output big (0x1A, 0x1F)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 9 | AmountIn  | 
| 32 | PairInitHash  | 
> AmountIn will be multiplied by 10^12

## Tests

```console
forge test --rpc-url <your-rpc-url-here>
```
## Benchmarks
!todo
