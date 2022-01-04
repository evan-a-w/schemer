(defmacro (name arg1)
    (car (cdr arg1)))

(name ((display 1) (display 2) (display 3)))
