def recursive():
    callee()


def callee():
    other_callee()


def other_callee():
    recursive()
