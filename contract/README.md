# Rusty-Sando/Contract ![license](https://img.shields.io/badge/License-MIT-green.svg?label=license)

Gas-optimized sando contract written in Huff to make use of unconventional gas optimizations. 

> Why not Yul? Yul does not give access to the stack or jump instructions. 

## Gas Optimizations

### JUMPDEST Function Sig
Instead of reserving 4 bytes for a function selector, store a JUMPDEST in the first byte of calldata and jump to it at the beginning of execution. Doing so allows us to jump to the code range 0x00-0xFF, so we fill that range with place holder JUMPDEST that point to the location of the associated function body. 

Example:
```as
#define macro MAIN() = takes (0) returns (0) {
    // extract function selector (JUMPDEST encoding)
    push0                                       // [0x00]
    calldataload                                // [calldata]
    push0                                       // [0x00, calldata]
    byte                                        // [jumplabel]
    jump                                        // []
```

> **Note**
> JUMPDEST 0xfa is reserved to handle [UniswapV3 callback](https://docs.uniswap.org/contracts/v3/reference/core/interfaces/callback/IUniswapV3SwapCallback).

### Encoding WETH Value Using tx.value
When dealing with WETH amounts, the amount is encoded by first dividing the value by 100000, and setting the divided value as `tx.value` when calling the contract. The contract then multiplies `tx.value` by 100000 to get the original amount. 

> The last 5 digits of the original value are lost after encoding, however, it is a small amount of wei and can be ignored.

Example:
```as
    // setup calldata for swap(wethOut, 0, address(this), "")
    [V2_Swap_Sig] 0x00 mstore                   
    0x0186a0 callvalue mul 0x04 mstore          // original weth value is decoded here by doing `100000 * callvalue`   
    0x00 0x24 mstore                   
    address 0x44 mstore                         
    0x80 0x64 mstore                     
```

### Encoding Other Token Value Using 5 Bytes Of Calldata
When dealing with the other token amount, the values can range significantly depending on the token decimal and total supply. To account for the full range, we encode by fitting the value into 4 bytes of calldata plus a byte shift. To decode, we byteshift the 4bytes to the left. 

We use byte shifts instead of bitshifts because we perform a byteshift by storing the 4bytes in memory N bytes to the left of its memory slot. 

To optimize further, instead of encoding the byteshift into our calldata, we encode the offset in memory such that when the 4bytes are stored, it will be N bytes from the left of its storage slot. [more detail](https://github.com/mouseless-eth/rusty-sando/blob/1a0f775a00ae932f64d7e926605134892fcf56f9/contract/test/misc/V2SandoUtility.sol#L28).

> **Note** 
> Free alfa: Might be able to optimize contract by eliminating unnecessary [memory expansions](https://www.evm.codes/about#memoryexpansion) by changing order that params are stored in memory. I did not account for this when writing the contract. 

### Hardcoded values
Weth address is hardcoded into the contract and there are individual methods to handle when Weth is token0 or token1. 

### Encode Packed
All calldata is encoded by packing the values together. 

## Interface

| JUMPDEST  | Function Name |
| :-------------: | :------------- |
| 0x05  | V2 Backrun, Weth is Token0 and Output  |
| 0x0A  | V2 Frontrun, Weth is Token0 and Input  |
| 0x0F  | V2 Backrun, Weth is Token1 and Output  |
| 0x14  | V2 Frontrun, Weth is Token1 and Input |
| 0x19  | V3 Backrun, Weth is Token1 and Output, Big Encoding |
| 0x1E  | V3 Backrun, Weth is Token0 and Output, Big Encoding  |
| 0x23  | V3 Backrun, Weth is Token1 and Output, Small Encoding  |
| 0x28  | V3 Backrun, Weth is Token0 and Output, Small Encoding |
| 0x2D  | V3 Frontrun, Weth is Token0 and Input  |
| 0x32  | V3 Frontrun, Weth is Token1 and Input  |
| 0x37  | Seppuku (self-destruct)  |
| 0x3C  | Recover Eth  |
| 0x41  | Recover Weth  |
| ...  | ...  |
| 0xFA  | UniswapV3 Callback  |


## Calldata Encoding (Interface)
### Uniswap V2 Calldata Encoding Format

#### Frontrun (weth is input)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 1 | Where to store AmountOut  |
| 4 | AmountOut  |

#### Backrun(weth is output)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 1 | Where to store AmountIn  |
| 4 | AmountIn  |

### Uniswap V3 Calldata Encoding Format

#### Frontrun (weth is input)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 32 | PairInitHash  | 
> PairInitHash used to verify msg.sender is pool in callback

#### Backrun (weth is output, small)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 6 | AmountIn  | 
| 32 | PairInitHash  | 
> Small encoding when AmountIn < 10^12

#### Backrun (weth is output, big)
| Byte Length  | Variable |
| :-------------: | :------------- |
| 1 | JUMPDEST  |
| 20 | PairAddress  |
| 20 | TokenInAddress  |
| 9 | AmountIn  | 
| 32 | PairInitHash  | 
> AmountIn will be multiplied by 10^12

## Running Tests

```console
forge install
forge test
```
