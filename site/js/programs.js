export const PROGRAMS = {
    factorial: {
        "Statement": {
            "Block": [
                {
                    "Function": {
                        "name": "factorial",
                        "inputs": [["n", "Integer"]],
                        "output": "Integer",
                        "body": {
                            "hash": "ignored_in_wasm_eval",
                            "content": {
                                "Statement": {
                                    "If": {
                                        "condition": {
                                            "Expression": {
                                                "Call": {
                                                    "function_hash": "intrinsic_lt",
                                                    "args": [
                                                        { "Variable": "n" },
                                                        { "Literal": "2" }
                                                    ]
                                                }
                                            }
                                        },
                                        "then_block": [
                                            { "Return": { "Literal": "1" } }
                                        ],
                                        "else_block": [
                                            {
                                                "Return": {
                                                    "Expression": {
                                                        "Call": {
                                                            "function_hash": "intrinsic_mul",
                                                            "args": [
                                                                { "Variable": "n" },
                                                                {
                                                                    "Expression": {
                                                                        "Call": {
                                                                            "function_hash": "factorial",
                                                                            "args": [
                                                                                {
                                                                                    "Expression": {
                                                                                        "Call": {
                                                                                            "function_hash": "intrinsic_sub",
                                                                                            "args": [
                                                                                                { "Variable": "n" },
                                                                                                { "Literal": "1" }
                                                                                            ]
                                                                                        }
                                                                                    }
                                                                                }
                                                                            ]
                                                                        }
                                                                    }
                                                                }
                                                            ]
                                                        }
                                                    }
                                                }
                                            }
                                        ]
                                    }
                                }
                            }
                        }
                    }
                },
                {
                    "Expression": {
                        "Call": {
                            "function_hash": "intrinsic_print",
                            "args": [
                                {
                                    "Expression": {
                                        "Call": {
                                            "function_hash": "factorial",
                                            "args": [{ "Literal": "5" }]
                                        }
                                    }
                                }
                            ]
                        }
                    }
                }
            ]
        }
    }
};
